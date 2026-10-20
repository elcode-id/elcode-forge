//! `eforge scan` command — full and incremental repository intelligence pipeline.

use std::collections::BTreeMap;
use std::path::PathBuf;

use forge_analyzer::{
    dep_health::analyze_npm_dependencies, graph::build_ts_architecture_graph,
    scanner::scan_repository,
};
use forge_core::{
    cache::{CacheMetadata, ForgeCache, EFORGE_DIR},
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
    let arch_graph = build_ts_architecture_graph(&opts.repo_root, &discovery.files)?;
    let dep_report = analyze_npm_dependencies(&opts.repo_root, &arch_graph.external_imports)?;

    let current_hashes: BTreeMap<String, String> = discovery
        .files
        .iter()
        .map(|f| (f.relative_path.clone(), f.content_hash.clone()))
        .collect();

    let config_serialized = serde_json::to_vec(&config).unwrap_or_default();
    let config_hash = ForgeCache::hash_bytes(&config_serialized);

    let files_for_security: Vec<FileNode> = if opts.incremental {
        let cache = ForgeCache::init(&opts.repo_root)?;
        let changed_paths = cache.diff_changed_files(&current_hashes, &config_hash);
        let meta = CacheMetadata {
            analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
            config_hash,
            file_hashes: current_hashes,
        };
        cache.save_metadata(&meta)?;

        if changed_paths.is_empty() {
            discovery.files.clone()
        } else {
            discovery
                .files
                .iter()
                .filter(|f| changed_paths.binary_search(&f.relative_path).is_ok())
                .cloned()
                .collect()
        }
    } else {
        if opts.repo_root.join(EFORGE_DIR).is_dir() {
            let cache = ForgeCache::init(&opts.repo_root)?;
            let meta = CacheMetadata {
                analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
                config_hash,
                file_hashes: current_hashes,
            };
            cache.save_metadata(&meta)?;
        }
        discovery.files.clone()
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
        modules: arch_graph.modules,
        dependencies: dep_report.dependencies,
        findings: Vec::new(),
        health: None,
    };

    let rule_engine = RuleEngine::from_config(&opts.repo_root, &config)?;
    let mut findings: Vec<Finding> = Vec::new();
    findings.extend(dep_report.findings);
    findings.extend(rule_engine.evaluate(&repo)?);
    findings.extend(SecurityScanner::scan_repository_files(
        &opts.repo_root,
        &files_for_security,
    )?);

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
}
