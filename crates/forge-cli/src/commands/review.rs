//! `eforge review` command — PR/change risk, blast radius, impacted test selection, and diff-scoped review report.

use std::collections::BTreeSet;
use std::path::Path;

use forge_core::{
    config::load_config,
    scoring::{calculate_change_risk, RiskReport},
    Finding, ForgeError,
};
use forge_git::ChangeImpactReport;
use serde::{Deserialize, Serialize};

use crate::cli::OutputFormat;
use crate::commands::diff::execute_diff;
use crate::commands::scan::{execute_scan, ScanOptions};
use crate::exit_code::ExitCode;

/// Complete change review report emitted by `eforge review`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewReport {
    /// Explainable change risk assessment (`Low|Medium|High|Critical` + score + reasons).
    pub risk: RiskReport,
    /// Blast-radius impact report across modules and test coverage.
    pub impact: ChangeImpactReport,
    /// Findings relevant to the reviewed change set.
    pub findings: Vec<Finding>,
}

/// Executes `eforge review` against `repo_root` with optional unified `diff_input` and `fast_mode`.
pub fn execute_review(
    repo_root: &Path,
    diff_input: Option<&str>,
    fast_mode: bool,
) -> Result<(ReviewReport, ExitCode), ForgeError> {
    let config = load_config(repo_root)?;
    let impact = execute_diff(repo_root, diff_input)?;

    let scan_opts = ScanOptions {
        repo_root: repo_root.to_path_buf(),
        incremental: fast_mode && repo_root.join(forge_core::cache::EFORGE_DIR).is_dir(),
        format: OutputFormat::Json,
    };
    let (repo, _) = execute_scan(&scan_opts)?;

    // If fast_mode is enabled and there are explicit file changes in the diff, scope findings
    // to the changed files or affected module roots.
    let changed_paths: BTreeSet<&str> = impact
        .changed_files
        .iter()
        .map(|c| c.path.as_str())
        .collect();
    let affected_modules: BTreeSet<&str> = impact
        .directly_affected_modules
        .iter()
        .chain(impact.transitively_affected_modules.iter())
        .map(String::as_str)
        .collect();

    let findings: Vec<Finding> = if fast_mode && !changed_paths.is_empty() {
        repo.findings
            .into_iter()
            .filter(|finding| {
                let Some(loc) = &finding.location else {
                    return true;
                };
                if changed_paths.contains(loc.file_path.as_str()) {
                    return true;
                }
                affected_modules
                    .iter()
                    .any(|module_id| loc.file_path.starts_with(&format!("{module_id}/")))
            })
            .collect()
    } else {
        repo.findings
    };

    let affected_modules_count =
        impact.directly_affected_modules.len() + impact.transitively_affected_modules.len();
    let missing_tests_count = impact.changed_files_without_tests.len();
    let risk = calculate_change_risk(
        &findings,
        impact.changed_files.len(),
        affected_modules_count,
        missing_tests_count,
    );
    let exit_code = ExitCode::evaluate_findings(&findings, config.fail_on_severity);

    Ok((
        ReviewReport {
            risk,
            impact,
            findings,
        },
        exit_code,
    ))
}

/// Renders a [`ReviewReport`] in the requested [`OutputFormat`].
pub fn render_review_report(
    report: &ReviewReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report).map_err(|err| ForgeError::Serialization {
                message: err.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut md = String::new();
            md.push_str("# ELCODE Forge Review Report\n\n");
            md.push_str(&format!(
                "- **Change Risk Level:** **{:?}** ({}/100)\n",
                report.risk.level, report.risk.score
            ));
            md.push_str(&format!(
                "- **Changed Files:** {}\n- **Directly Affected Modules:** `{}`\n- **Transitively Affected Modules:** `{}`\n- **Changed Files Without Tests:** {}\n\n",
                report.impact.changed_files.len(),
                if report.impact.directly_affected_modules.is_empty() {
                    "none".to_string()
                } else {
                    report.impact.directly_affected_modules.join(", ")
                },
                if report.impact.transitively_affected_modules.is_empty() {
                    "none".to_string()
                } else {
                    report.impact.transitively_affected_modules.join(", ")
                },
                report.impact.changed_files_without_tests.len()
            ));

            if !report.risk.reasons.is_empty() {
                md.push_str("## Risk Signals\n\n");
                for reason in &report.risk.reasons {
                    md.push_str(&format!("- {reason}\n"));
                }
                md.push('\n');
            }

            md.push_str(&format!(
                "## Review Findings ({})\n\n",
                report.findings.len()
            ));
            if report.findings.is_empty() {
                md.push_str("_No findings triggered by this change._\n");
            } else {
                md.push_str(
                    "| Severity | Rule ID | Location | Title |\n| :--- | :--- | :--- | :--- |\n",
                );
                for finding in &report.findings {
                    let loc = finding
                        .location
                        .as_ref()
                        .map(|l| match l.start_line {
                            Some(line) => format!("`{}:{}`", l.file_path, line),
                            None => format!("`{}`", l.file_path),
                        })
                        .unwrap_or_else(|| "`-`".to_string());
                    md.push_str(&format!(
                        "| **{:?}** | `{}` | {} | {} |\n",
                        finding.severity, finding.rule_id, loc, finding.title
                    ));
                }
            }
            Ok(md)
        }
        OutputFormat::Terminal => {
            let theme = crate::theme::Theme::default();
            let mut out = String::new();
            out.push_str("============================================================\n");
            let level_str = format!("{:?}", report.risk.level);
            let styled_level = match report.risk.level {
                forge_core::scoring::RiskLevel::Critical => theme.red(&level_str),
                forge_core::scoring::RiskLevel::High => theme.red(&level_str),
                forge_core::scoring::RiskLevel::Medium => theme.yellow(&level_str),
                forge_core::scoring::RiskLevel::Low => theme.green(&level_str),
            };
            out.push_str(&format!(
                " {}\n",
                theme.bold(&format!(
                    "ELCODE Forge Review — Risk: {} ({}/100)",
                    styled_level, report.risk.score
                ))
            ));
            out.push_str("============================================================\n");
            out.push_str(&format!(
                "Changed Files:                 {}\n",
                report.impact.changed_files.len()
            ));
            out.push_str(&format!(
                "Directly Affected Modules:     {}\n",
                if report.impact.directly_affected_modules.is_empty() {
                    "none".to_string()
                } else {
                    report.impact.directly_affected_modules.join(", ")
                }
            ));
            out.push_str(&format!(
                "Transitively Affected Modules: {}\n",
                if report.impact.transitively_affected_modules.is_empty() {
                    "none".to_string()
                } else {
                    report.impact.transitively_affected_modules.join(", ")
                }
            ));
            out.push_str(&format!(
                "Missing Tests on Changed Code: {}\n\n",
                report.impact.changed_files_without_tests.len()
            ));

            if !report.risk.reasons.is_empty() {
                out.push_str("Risk Reasons:\n");
                for reason in &report.risk.reasons {
                    out.push_str(&format!("  - {reason}\n"));
                }
                out.push('\n');
            }

            out.push_str(&format!("Findings ({}):\n", report.findings.len()));
            if report.findings.is_empty() {
                out.push_str("  No findings detected.\n");
            } else {
                for finding in &report.findings {
                    out.push_str(&crate::formatters::terminal::render_finding_card(
                        finding, &theme,
                    ));
                    out.push('\n');
                }
            }
            Ok(out)
        }
    }
}

/// Computes the Tier 1/2/3 impacted test suite and recommended runner commands for `eforge review --impacted-tests`.
pub fn execute_review_impacted_tests(
    repo_root: &Path,
    diff_input: Option<&str>,
) -> Result<forge_git::ImpactedTestsReport, ForgeError> {
    let config = load_config(repo_root)?;
    let discovery = forge_analyzer::scanner::scan_repository(repo_root, &config.ignore_patterns)?;
    let graph = forge_analyzer::build_polyglot_architecture_graph(repo_root, &discovery.files)?;
    let file_edges = forge_analyzer::extract_file_import_edges(repo_root, &discovery.files)?;
    let diff_text = match diff_input {
        Some(raw) => raw.to_string(),
        None => forge_git::collect_working_tree_diff(repo_root).unwrap_or_default(),
    };
    let changes = forge_git::parse_unified_diff(&diff_text)?;
    let test_files: BTreeSet<String> = discovery
        .files
        .iter()
        .filter(|f| forge_analyzer::detector::is_test_file(&f.relative_path))
        .map(|f| f.relative_path.clone())
        .collect();

    Ok(forge_git::compute_impacted_tests(
        &changes,
        &graph.modules,
        &file_edges,
        &test_files,
        &discovery.languages,
        &discovery.package_managers,
    ))
}

/// Renders an [`forge_git::ImpactedTestsReport`] appendix for `eforge review --impacted-tests`.
#[must_use]
pub fn render_impacted_tests_appendix(
    report: &forge_git::ImpactedTestsReport,
    format: OutputFormat,
) -> String {
    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report).unwrap_or_default()
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut md = String::from("\n## Smart Impacted Test Selection\n\n");
            md.push_str(&format!(
                "- **Total Selected Tests:** {}\n- **Tier 1 (Modified):** `{}`\n- **Tier 2 (Direct Importers/Companions):** `{}`\n- **Tier 3 (Transitive Blast Radius):** `{}`\n\n",
                report.all_selected_tests.len(),
                report.tier1_modified_tests.join(", "),
                report.tier2_direct_tests.join(", "),
                report.tier3_transitive_tests.join(", ")
            ));
            for cmd in &report.recommended_commands {
                md.push_str(&format!("```bash\n{cmd}\n```\n"));
            }
            md
        }
        OutputFormat::Terminal => {
            let mut out = String::from("\nSmart Impacted Test Selection:\n");
            out.push_str(&format!(
                "  Selected Test Files ({}): {}\n",
                report.all_selected_tests.len(),
                if report.all_selected_tests.is_empty() {
                    "none".to_string()
                } else {
                    report.all_selected_tests.join(", ")
                }
            ));
            for cmd in &report.recommended_commands {
                out.push_str(&format!("  Recommended Runner: {cmd}\n"));
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use forge_core::{cache::EFORGE_DIR, scoring::RiskLevel};

    use super::{execute_review, render_review_report};
    use crate::cli::OutputFormat;

    fn fixture_next_app_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app")
    }

    #[test]
    fn computes_blast_radius_risk_and_renders_markdown_review_report() {
        let root = fixture_next_app_dir();
        let sample_diff = "\
--- a/src/db/client.ts
+++ b/src/db/client.ts
@@ -1,3 +1,5 @@
+export const poolSize = 10;
+export const timeoutMs = 5000;
";
        let res = execute_review(&root, Some(sample_diff), true);
        let _ = fs::remove_dir_all(root.join(EFORGE_DIR));
        assert!(res.is_ok());

        if let Ok((report, _)) = res {
            assert_eq!(report.impact.changed_files.len(), 1);
            assert!(report
                .impact
                .directly_affected_modules
                .contains(&"src/db".to_string()));
            assert!(!report.impact.transitively_affected_modules.is_empty());
            assert!(report.risk.level >= RiskLevel::Medium);

            let md_res = render_review_report(&report, OutputFormat::Markdown);
            assert!(md_res.is_ok());
            if let Ok(md) = md_res {
                assert!(md.contains("# ELCODE Forge Review Report"));
                assert!(md.contains("src/db"));
            }

            let term_res = render_review_report(&report, OutputFormat::Terminal);
            assert!(term_res.is_ok());
            if let Ok(term_text) = term_res {
                assert!(term_text.contains("ELCODE Forge Review"));
                assert!(term_text.contains("Changed Files:"));
                assert!(term_text.contains("Directly Affected Modules:"));
            }
        }
    }
}
