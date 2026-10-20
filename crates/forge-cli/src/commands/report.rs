use std::fs;
use std::path::Path;

use forge_analyzer::{graph::build_ts_architecture_graph, scanner::scan_repository};
use forge_core::{
    config::load_config,
    scoring::{calculate_change_risk, RiskReport},
    ForgeError, Repository, Severity,
};
use forge_git::{
    collect_commit_log, compute_hotspots_from_log, read_head_revision, ChangeImpactReport,
    GitRevision, HotspotEntry,
};
use forge_rules::RuleEngine;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::commands::diff::execute_diff;
use crate::commands::scan::{execute_scan, ScanOptions};

/// Schema version for [`ReportBundle`] (version 1).
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// High-level architecture summary for the dashboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ArchitectureSummary {
    pub circular_dependencies: Vec<Vec<String>>,
    pub high_centrality_modules: Vec<String>,
}

/// Rule metadata embedded in [`ReportBundle`] for rules catalog and violation linkage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RuleDescriptor {
    pub id: String,
    pub name: String,
    pub category: String,
    pub severity: Severity,
    pub source: String,
    pub enabled: bool,
    pub description: String,
    pub remediation: Option<String>,
}

/// Versioned, deterministic intelligence bundle consumable by the web dashboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReportBundle {
    pub schema_version: u32,
    pub tool_version: String,
    pub revision: Option<GitRevision>,
    pub repository: Repository,
    pub architecture: ArchitectureSummary,
    pub hotspots: Vec<HotspotEntry>,
    pub change_impact: Option<ChangeImpactReport>,
    pub change_risk: Option<RiskReport>,
    pub rules: Vec<RuleDescriptor>,
}

/// Options configuring [`build_report_bundle`].
#[derive(Debug, Clone)]
pub struct ReportOptions {
    pub hotspots_top_n: usize,
    pub include_git: bool,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            hotspots_top_n: 10,
            include_git: true,
        }
    }
}

/// Builds a deterministic [`ReportBundle`] from the given repository root.
pub fn build_report_bundle(
    repo_root: &Path,
    opts: &ReportOptions,
) -> Result<ReportBundle, ForgeError> {
    let scan_opts = ScanOptions {
        repo_root: repo_root.to_path_buf(),
        incremental: false,
        format: crate::cli::OutputFormat::Json,
    };
    let (repo, _) = execute_scan(&scan_opts)?;

    let discovery = scan_repository(repo_root, &[])?;
    let arch_graph = build_ts_architecture_graph(repo_root, &discovery.files)?;

    let architecture = ArchitectureSummary {
        circular_dependencies: arch_graph.circular_dependencies,
        high_centrality_modules: arch_graph.high_centrality_modules,
    };

    let (revision, hotspots, change_impact, change_risk) = if opts.include_git {
        let rev = read_head_revision(repo_root)?;
        let log_text = collect_commit_log(repo_root, 100)?;
        let hs = compute_hotspots_from_log(&log_text, opts.hotspots_top_n);

        let impact = execute_diff(repo_root, None).ok();
        let risk = impact.as_ref().map(|imp| {
            let total_affected =
                imp.directly_affected_modules.len() + imp.transitively_affected_modules.len();
            let missing_tests = imp.changed_files_without_tests.len();
            calculate_change_risk(
                &repo.findings,
                imp.changed_files.len(),
                total_affected,
                missing_tests,
            )
        });

        (rev, hs, impact, risk)
    } else {
        (None, Vec::new(), None, None)
    };

    let config = load_config(repo_root)?;
    let engine = RuleEngine::from_config(repo_root, &config)?;

    let mut rules = Vec::new();

    // 1. Built-in rules
    for r in engine.rules() {
        let (name, category, desc, rem) = match r.id.as_str() {
            "arch/no-direct-db-in-ui" => (
                "No Direct DB in UI",
                "architecture",
                "UI components must not import database modules directly.",
                Some(
                    "Route database queries through service layer or data access repositories."
                        .to_string(),
                ),
            ),
            "arch/no-circular-dependencies" => (
                "No Circular Dependencies",
                "architecture",
                "Module import graph must be an acyclic directed graph (DAG).",
                Some("Extract shared interfaces or decouple circular dependencies.".to_string()),
            ),
            "quality/require-service-tests" => (
                "Require Service Tests",
                "quality",
                "Application services should have companion test files.",
                Some("Create companion test file in tests/ directory.".to_string()),
            ),
            "quality/max-file-lines" => (
                "Max File Lines",
                "quality",
                "Source files should not exceed configured line limits.",
                Some("Refactor and decompose file into smaller modular units.".to_string()),
            ),
            _ => ("Built-in Rule", "quality", "Engine rule", None),
        };

        rules.push(RuleDescriptor {
            id: r.id.clone(),
            name: name.to_string(),
            category: category.to_string(),
            severity: r.severity,
            source: "builtin".to_string(),
            enabled: r.enabled,
            description: desc.to_string(),
            remediation: rem,
        });
    }

    // 2. Declarative rules
    for d in engine.declarative_rules() {
        rules.push(RuleDescriptor {
            id: d.id.clone(),
            name: d.name.clone(),
            category: d.category.as_str().to_string(),
            severity: d.severity,
            source: d.source.to_string(),
            enabled: d.enabled,
            description: d.description.clone(),
            remediation: if d.remediation.is_empty() {
                None
            } else {
                Some(d.remediation.clone())
            },
        });
    }

    // Deterministic sort: Severity descending, then ID ascending
    rules.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));

    Ok(ReportBundle {
        schema_version: REPORT_SCHEMA_VERSION,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        revision,
        repository: repo,
        architecture,
        hotspots,
        change_impact,
        change_risk,
        rules,
    })
}

/// Executes the `eforge report` CLI subcommand, writing JSON output to stdout or an optional file.
pub fn run_report(
    repo_root: &Path,
    output: Option<&Path>,
    hotspots_top_n: usize,
    no_git: bool,
    save_history: bool,
) -> Result<String, ForgeError> {
    let opts = ReportOptions {
        hotspots_top_n,
        include_git: !no_git,
    };
    let bundle = build_report_bundle(repo_root, &opts)?;
    let rendered =
        serde_json::to_string_pretty(&bundle).map_err(|e| ForgeError::Serialization {
            message: e.to_string(),
        })?;

    if save_history {
        let _ = crate::commands::history::save_history_snapshot(repo_root, &bundle)?;
    }

    if let Some(out_path) = output {
        fs::write(out_path, &rendered).map_err(|source| ForgeError::Io {
            path: out_path.to_path_buf(),
            source,
        })?;
    }

    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_next_app() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app")
    }

    #[test]
    fn golden_determinism_test() {
        let fixture = fixture_next_app();
        let opts = ReportOptions {
            hotspots_top_n: 10,
            include_git: false,
        };

        let bundle1_res = build_report_bundle(&fixture, &opts);
        let bundle2_res = build_report_bundle(&fixture, &opts);
        assert!(bundle1_res.is_ok());
        assert!(bundle2_res.is_ok());

        if let (Ok(bundle1), Ok(bundle2)) = (bundle1_res, bundle2_res) {
            let json1_res = serde_json::to_string_pretty(&bundle1);
            let json2_res = serde_json::to_string_pretty(&bundle2);
            assert!(json1_res.is_ok());
            assert!(json2_res.is_ok());

            if let (Ok(json1), Ok(json2)) = (json1_res, json2_res) {
                assert_eq!(
                    json1, json2,
                    "ReportBundle JSON must be byte-for-byte identical"
                );
                assert_eq!(bundle1.schema_version, 1);
                assert_eq!(bundle1.tool_version, env!("CARGO_PKG_VERSION"));

                // Verify circular dependencies from fixture
                assert!(
                    bundle1
                        .architecture
                        .circular_dependencies
                        .iter()
                        .any(|cycle| cycle.contains(&"src/services".to_string())
                            && cycle.contains(&"src/db".to_string())),
                    "ReportBundle must contain circular dependencies"
                );

                // Verify rules are populated and sorted
                assert!(
                    !bundle1.rules.is_empty(),
                    "Rules must be populated in ReportBundle"
                );
                assert!(
                    bundle1
                        .rules
                        .iter()
                        .any(|r| r.id == "arch/no-direct-db-in-ui"),
                    "Built-in arch rule must exist in bundle rules"
                );
            }
        }
    }
}
