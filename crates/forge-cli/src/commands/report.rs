use std::fs;
use std::path::Path;

use forge_analyzer::{graph::build_ts_architecture_graph, scanner::scan_repository};
use forge_core::{
    scoring::{calculate_change_risk, RiskReport},
    ForgeError, Repository,
};
use forge_git::{
    collect_commit_log, compute_hotspots_from_log, read_head_revision, ChangeImpactReport,
    GitRevision, HotspotEntry,
};
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

    Ok(ReportBundle {
        schema_version: REPORT_SCHEMA_VERSION,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        revision,
        repository: repo,
        architecture,
        hotspots,
        change_impact,
        change_risk,
    })
}

/// Executes the `eforge report` CLI subcommand, writing JSON output to stdout or an optional file.
pub fn run_report(
    repo_root: &Path,
    output: Option<&Path>,
    hotspots_top_n: usize,
    no_git: bool,
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
            }
        }
    }
}
