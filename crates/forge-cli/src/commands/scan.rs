//! `eforge scan` command — full and incremental repository intelligence pipeline.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use forge_analyzer::{
    analyze_npm_dependencies, analyze_python_dependencies, build_go_architecture_graph,
    build_python_architecture_graph, build_rust_architecture_graph, build_ts_architecture_graph,
    parse_go_manifest, parse_rust_workspace, scan_repository,
};
use forge_core::{
    cache::{CacheMetadata, ForgeCache, EFORGE_DIR},
    config::load_config,
    scoring::calculate_health_score,
    Confidence, FileNode, Finding, ForgeError, Repository, Severity,
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

    let has_py = discovery
        .files
        .iter()
        .any(|f| f.relative_path.ends_with(".py"));
    let has_ts_js = discovery.files.iter().any(|f| {
        matches!(
            f.language.as_deref(),
            Some("typescript") | Some("javascript") | Some("vue") | Some("svelte")
        )
    });
    let has_go = discovery
        .files
        .iter()
        .any(|f| matches!(f.language.as_deref(), Some("go")) || f.relative_path.ends_with(".go"));
    let has_rust = discovery
        .files
        .iter()
        .any(|f| matches!(f.language.as_deref(), Some("rust")) || f.relative_path.ends_with(".rs"));

    let mut modules = Vec::new();
    let mut external_imports = BTreeSet::new();

    if has_ts_js {
        let ts_graph = build_ts_architecture_graph(&opts.repo_root, &discovery.files)?;
        modules.extend(ts_graph.modules);
        external_imports.extend(ts_graph.external_imports);
    }
    if has_py {
        let py_graph = build_python_architecture_graph(&opts.repo_root, &discovery.files)?;
        modules.extend(py_graph.modules);
        external_imports.extend(py_graph.external_imports);
    }
    if has_go {
        let module_path = if opts.repo_root.join("go.mod").is_file() {
            parse_go_manifest(&opts.repo_root)
                .ok()
                .and_then(|m| m.module_path)
        } else {
            None
        };
        let go_graph =
            build_go_architecture_graph(&opts.repo_root, &discovery.files, module_path.as_deref())?;
        modules.extend(go_graph.modules);
        external_imports.extend(go_graph.external_imports);
    }
    if has_rust {
        let rust_graph = build_rust_architecture_graph(&opts.repo_root, &discovery.files)?;
        modules.extend(rust_graph.modules);
        external_imports.extend(rust_graph.external_imports);
    }

    let mut dependencies = Vec::new();
    let mut dep_findings = Vec::new();

    if opts.repo_root.join("package.json").is_file() {
        let npm_report = analyze_npm_dependencies(&opts.repo_root, &external_imports)?;
        dependencies.extend(npm_report.dependencies);
        dep_findings.extend(npm_report.findings);
    }

    if opts.repo_root.join("pyproject.toml").is_file()
        || opts.repo_root.join("requirements.txt").is_file()
        || opts.repo_root.join("poetry.lock").is_file()
        || opts.repo_root.join("uv.lock").is_file()
    {
        let py_report = analyze_python_dependencies(&opts.repo_root, &external_imports)?;
        dependencies.extend(py_report.dependencies);
        dep_findings.extend(py_report.findings);
    }

    if opts.repo_root.join("go.mod").is_file() {
        if let Ok(go_manifest) = parse_go_manifest(&opts.repo_root) {
            dependencies.extend(go_manifest.dependencies);
        }
    }

    if opts.repo_root.join("Cargo.toml").is_file() {
        if let Ok(rust_ws) = parse_rust_workspace(&opts.repo_root) {
            dependencies.extend(rust_ws.dependencies);
            for wild in rust_ws.wildcard_dependencies {
                dep_findings.push(Finding {
                    id: format!("dep/cargo-wildcard-{wild}"),
                    category: "dependencies".to_string(),
                    severity: Severity::High,
                    confidence: Confidence::Certain,
                    title: format!("Wildcard dependency constraint on '{wild}'"),
                    description: format!(
                        "Package '{wild}' uses wildcard constraint '*' in Cargo manifest. This allows untested breaking versions to be pulled in."
                    ),
                    evidence: format!("Cargo.toml: {wild} = \"*\""),
                    location: None,
                    recommendation: "Pin the dependency to a specific version requirement.".to_string(),
                    rule_id: "deps/unpinned-version".to_string(),
                });
            }
        }
    }

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
        modules,
        dependencies,
        findings: Vec::new(),
        health: None,
    };

    let rule_engine = RuleEngine::from_config(&opts.repo_root, &config)?;
    let mut findings: Vec<Finding> = Vec::new();
    findings.extend(dep_findings);
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
