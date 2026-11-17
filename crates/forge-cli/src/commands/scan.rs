//! `eforge scan` command — full and incremental repository intelligence pipeline.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use forge_analyzer::{
    analyze_polyglot_dependencies_cached, build_polyglot_architecture_graph_cached, scan_repository,
};
use forge_core::{
    cache::{CacheMetadata, CachedFileFindings, FileFindingsCache, ForgeCache, EFORGE_DIR},
    config::load_config,
    scoring::calculate_health_score,
    FileNode, Finding, ForgeError, Repository,
};
use forge_rules::RuleEngine;
use forge_security::SecurityScanner;

use crate::cli::OutputFormat;
use crate::exit_code::ExitCode;

/// Configuration options for [`execute_scan`].
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// Target repository root directory.
    pub repo_root: PathBuf,
    /// Whether to use `.eforge/metadata.json` BLAKE3 file hashes for incremental scanning.
    pub incremental: bool,
    /// Desired output format.
    pub format: OutputFormat,
}

/// Executes the end-to-end `eforge scan` pipeline and returns the populated [`Repository`] IR
/// alongside the policy [`ExitCode`].
pub fn execute_scan(opts: &ScanOptions) -> Result<(Repository, ExitCode), ForgeError> {
    let config = load_config(&opts.repo_root)?;
    let discovery = scan_repository(&opts.repo_root, &config.ignore_patterns)?;

    let current_hashes: BTreeMap<String, String> = discovery
        .files
        .iter()
        .map(|f| (f.relative_path.clone(), f.content_hash.clone()))
        .collect();

    let config_serialized = serde_json::to_vec(&config).unwrap_or_default();
    let config_hash = ForgeCache::hash_bytes(&config_serialized);
    let cache_opt = ForgeCache::init(&opts.repo_root).ok();

    let arch_graph = build_polyglot_architecture_graph_cached(
        &opts.repo_root,
        &discovery.files,
        cache_opt.as_ref(),
        None,
        &config_hash,
    )?;
    let dep_report = analyze_polyglot_dependencies_cached(
        &opts.repo_root,
        &discovery.files,
        &arch_graph.external_imports,
        cache_opt.as_ref(),
        None,
        &config_hash,
    )?;

    let modules = arch_graph.modules;
    let dependencies = dep_report.dependencies;
    let dep_findings = dep_report.findings;

    let (files_for_security, file_findings_cache) = if opts.incremental {
        let cache = ForgeCache::init(&opts.repo_root)?;
        let _changed_paths = cache.diff_changed_files(&current_hashes, &config_hash);
        let meta = CacheMetadata {
            analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
            config_hash: config_hash.clone(),
            file_hashes: current_hashes,
        };
        cache.save_metadata(&meta)?;

        let mut findings_cache = FileFindingsCache::load(&opts.repo_root, &config_hash);
        let active_paths: BTreeSet<&str> = discovery
            .files
            .iter()
            .map(|f| f.relative_path.as_str())
            .collect();
        findings_cache.retain_files(&active_paths);

        let files_to_scan: Vec<FileNode> = discovery
            .files
            .iter()
            .filter(|f| match findings_cache.entries.get(&f.relative_path) {
                Some(entry) => entry.content_hash != f.content_hash,
                None => true,
            })
            .cloned()
            .collect();

        (files_to_scan, Some(findings_cache))
    } else {
        let findings_cache = if cache_opt.is_some() || opts.repo_root.join(EFORGE_DIR).is_dir() {
            if let Ok(cache) = ForgeCache::init(&opts.repo_root) {
                let meta = CacheMetadata {
                    analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
                    config_hash: config_hash.clone(),
                    file_hashes: current_hashes,
                };
                let _ = cache.save_metadata(&meta);
            }
            let mut cache = FileFindingsCache::load(&opts.repo_root, &config_hash);
            let active_paths: BTreeSet<&str> = discovery
                .files
                .iter()
                .map(|f| f.relative_path.as_str())
                .collect();
            cache.retain_files(&active_paths);
            Some(cache)
        } else {
            None
        };
        (discovery.files.clone(), findings_cache)
    };

    let repo_name = config.project_name.clone().unwrap_or_else(|| {
        opts.repo_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repository")
            .to_string()
    });

    let mut repo = Repository {
        root_path: opts.repo_root.to_string_lossy().replace('\\', "/"),
        name: repo_name,
        detected_languages: discovery.languages,
        detected_frameworks: discovery.frameworks,
        package_managers: discovery.package_managers,
        files: discovery.files,
        modules,
        dependencies,
        findings: Vec::new(),
        health: None,
    };

    let newly_scanned_findings = if files_for_security.is_empty() {
        Vec::new()
    } else {
        SecurityScanner::scan_repository_files_with_config(
            &opts.repo_root,
            &files_for_security,
            config.security.as_ref(),
        )?
    };

    let security_file_findings = if let Some(mut cache) = file_findings_cache {
        let mut by_file: BTreeMap<String, Vec<Finding>> = BTreeMap::new();
        for finding in newly_scanned_findings {
            if let Some(loc) = &finding.location {
                by_file
                    .entry(loc.file_path.clone())
                    .or_default()
                    .push(finding);
            }
        }
        for file in &files_for_security {
            let file_findings = by_file.remove(&file.relative_path).unwrap_or_default();
            cache.entries.insert(
                file.relative_path.clone(),
                CachedFileFindings {
                    relative_path: file.relative_path.clone(),
                    content_hash: file.content_hash.clone(),
                    findings: file_findings,
                },
            );
        }
        let _ = cache.save(&opts.repo_root);

        let mut all_file_findings = Vec::new();
        for entry in cache.entries.values() {
            all_file_findings.extend(entry.findings.clone());
        }
        for unassociated in by_file.into_values() {
            all_file_findings.extend(unassociated);
        }
        all_file_findings
    } else {
        newly_scanned_findings
    };

    let rule_engine = RuleEngine::from_config(&opts.repo_root, &config)?;
    let mut findings: Vec<Finding> = Vec::new();
    findings.extend(dep_findings);
    findings.extend(rule_engine.evaluate(&repo)?);
    findings.extend(security_file_findings);
    findings.extend(SecurityScanner::scan_dependencies(
        &opts.repo_root,
        &repo.dependencies,
    ));
    findings.extend(SecurityScanner::scan_license_compliance(
        &opts.repo_root,
        config.license.as_ref(),
        &repo.dependencies,
    ));

    // Deterministic sort: highest severity first, then lexical ID ascending
    findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
    findings.dedup_by(|a, b| a.id == b.id && a.rule_id == b.rule_id && a.location == b.location);

    repo.findings = findings;
    repo.health = Some(calculate_health_score(&repo));

    let exit_code = ExitCode::evaluate_findings(&repo.findings, config.fail_on_severity);

    Ok((repo, exit_code))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use forge_core::cache::EFORGE_DIR;

    use super::{execute_scan, ScanOptions};
    use crate::cli::OutputFormat;
    use crate::formatters::render_repository_report;

    fn fixture_next_app_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app")
    }

    #[test]
    fn executes_full_and_incremental_scan_on_fixture_next_app_deterministically() {
        let root = fixture_next_app_dir();
        let opts = ScanOptions {
            repo_root: root.clone(),
            incremental: false,
            format: OutputFormat::Json,
        };

        let res1 = execute_scan(&opts);
        let res2 = execute_scan(&opts);
        assert!(res1.is_ok());
        assert!(res2.is_ok());

        if let (Ok((repo1, _)), Ok((repo2, _))) = (res1, res2) {
            assert_eq!(repo1, repo2);
            assert!(repo1.health.is_some());
            assert!(repo1
                .findings
                .iter()
                .any(|f| f.rule_id == "arch/no-direct-db-in-ui"));
            assert!(repo1
                .findings
                .iter()
                .any(|f| f.rule_id == "arch/no-circular-dependencies"));

            let json1 = render_repository_report(&repo1, OutputFormat::Json);
            let json2 = render_repository_report(&repo2, OutputFormat::Json);
            assert!(json1.is_ok());
            assert_eq!(json1.ok(), json2.ok());
        }

        // Verify incremental scan writes .eforge/metadata.json and clean up afterwards
        let inc_opts = ScanOptions {
            repo_root: root.clone(),
            incremental: true,
            format: OutputFormat::Sarif,
        };
        let inc_res = execute_scan(&inc_opts);
        assert!(inc_res.is_ok());
        assert!(root.join(EFORGE_DIR).join("metadata.json").is_file());
        let _ = fs::remove_dir_all(root.join(EFORGE_DIR));
    }

    #[test]
    fn preserves_security_findings_in_incremental_scan_with_zero_or_single_file_change() {
        use forge_core::cache::FILE_FINDINGS_CACHE_FILENAME;

        let temp_dir = std::env::temp_dir().join("eforge_scan_incremental_security_test_994");
        let _ = fs::remove_dir_all(&temp_dir);
        let src_dir = temp_dir.join("src");
        assert!(fs::create_dir_all(&src_dir).is_ok());

        let cfg = "version: 1\nproject:\n  name: \"test-incremental-repo\"\n";
        assert!(fs::write(temp_dir.join("eforge.config.yaml"), cfg).is_ok());

        // File A contains a hardcoded API token/secret
        let secret_file = src_dir.join("secrets.ts");
        let secret_content = "export const token = 'ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';\n";
        assert!(fs::write(&secret_file, secret_content).is_ok());

        // File B is a plain untouched typescript file
        let index_file = src_dir.join("index.ts");
        let index_content = "export const greet = () => 'hello';\n";
        assert!(fs::write(&index_file, index_content).is_ok());

        // Step 1: Full scan populates FileFindingsCache
        let full_opts = ScanOptions {
            repo_root: temp_dir.clone(),
            incremental: false,
            format: OutputFormat::Json,
        };
        let full_res = execute_scan(&full_opts);
        assert!(full_res.is_ok());
        let Ok((full_repo, _)) = full_res else { return };
        let secret_findings_initial = full_repo
            .findings
            .iter()
            .filter(|f| f.rule_id.contains("secret") || f.category == "security")
            .cloned()
            .collect::<Vec<_>>();
        assert!(!secret_findings_initial.is_empty());
        assert!(temp_dir
            .join(EFORGE_DIR)
            .join("cache")
            .join(FILE_FINDINGS_CACHE_FILENAME)
            .is_file());

        // Step 2: Incremental scan with 0 changes retains identical findings
        let inc_opts = ScanOptions {
            repo_root: temp_dir.clone(),
            incremental: true,
            format: OutputFormat::Json,
        };
        let inc_res_0 = execute_scan(&inc_opts);
        assert!(inc_res_0.is_ok());
        let Ok((inc_repo_0, _)) = inc_res_0 else {
            return;
        };
        assert_eq!(inc_repo_0.findings, full_repo.findings);

        // Step 3: Modify index.ts (only 1 file changed; secrets.ts untouched)
        assert!(fs::write(
            &index_file,
            "export const greet = () => 'hello world modified';\n"
        )
        .is_ok());

        let inc_res_1 = execute_scan(&inc_opts);
        assert!(inc_res_1.is_ok());
        let Ok((inc_repo_1, _)) = inc_res_1 else {
            return;
        };
        let secret_findings_after = inc_repo_1
            .findings
            .iter()
            .filter(|f| f.rule_id.contains("secret") || f.category == "security")
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(secret_findings_initial, secret_findings_after);

        // Parity: Run full scan on modified repo and assert identical findings
        let full_mod_res = execute_scan(&full_opts);
        assert!(full_mod_res.is_ok());
        let Ok((full_repo_mod, _)) = full_mod_res else {
            return;
        };
        assert_eq!(inc_repo_1.findings, full_repo_mod.findings);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
