//! `eforge trend` and `eforge blame-arch` commands — Temporal architecture intelligence.

use std::path::Path;

use forge_analyzer::extract_file_import_edges;
use forge_core::ForgeError;
use forge_git::{
    analyze_health_trend, blame_architecture_violations, ArchBlameEntry, HealthTrendReport,
};

use crate::cli::OutputFormat;
use crate::commands::scan::{execute_scan, ScanOptions};
use crate::theme::Theme;

/// Executes `eforge trend` across the last `max_commits` commits.
pub fn execute_trend(
    repo_root: &Path,
    max_commits: usize,
) -> Result<HealthTrendReport, ForgeError> {
    let scan_opts = ScanOptions {
        repo_root: repo_root.to_path_buf(),
        incremental: true,
        format: OutputFormat::Json,
    };
    let (repo, _) = execute_scan(&scan_opts)?;
    let current_score = repo.health.as_ref().map_or(100, |h| h.overall);

    analyze_health_trend(repo_root, current_score, &repo.modules, max_commits)
}

/// Renders a [`HealthTrendReport`] in the requested [`OutputFormat`].
pub fn render_trend_report(
    report: &HealthTrendReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut md = String::new();
            md.push_str("# ELCODE Forge Temporal Health Trend\n\n");
            md.push_str(&format!(
                "- **Current Health Score:** {}/100\n- **Earliest Score in Window:** {}/100\n- **Net Delta:** {:+}\n- **Drift Status:** `{}`\n\n",
                report.current_health_score,
                report.earliest_health_score,
                report.net_health_delta,
                report.drift_status
            ));
            if !report.composite_hotspots.is_empty() {
                md.push_str("## Composite Risk Hotspots (Churn × Blast Radius)\n\n");
                md.push_str("| File | Module | Commits | Dependent Modules | Risk Score |\n| :--- | :--- | :---: | :---: | :---: |\n");
                for h in &report.composite_hotspots {
                    md.push_str(&format!(
                        "| `{}` | `{}` | {} | {} | **{}/100** |\n",
                        h.path,
                        h.module_id,
                        h.commit_count,
                        h.dependent_modules_count,
                        h.hotspot_risk_score
                    ));
                }
            }
            Ok(md)
        }
        OutputFormat::Terminal => {
            let theme = Theme::default();
            let mut out = String::new();
            out.push_str("============================================================\n");
            out.push_str(&format!(
                " {}\n",
                theme.bold("ELCODE Forge Temporal Architecture Trend")
            ));
            out.push_str("============================================================\n");
            out.push_str(&format!(
                "Current Health: {} │ Drift Status: {} ({:+})\n\n",
                theme.render_score_meter(report.current_health_score),
                theme.bold(&report.drift_status),
                report.net_health_delta
            ));

            if !report.timeline.is_empty() {
                out.push_str("Commit Health Evolution:\n");
                for pt in &report.timeline {
                    let delta_str = format!("{:+}", pt.score_delta);
                    out.push_str(&format!(
                        "  {}  [{:>3}/100 ({:>3})]  ({:>2} files)  {} — {}\n",
                        theme.cyan(&pt.short_sha),
                        pt.estimated_health_score,
                        delta_str,
                        pt.files_touched,
                        pt.subject,
                        theme.dim(&pt.author)
                    ));
                }
                out.push('\n');
            }

            if !report.composite_hotspots.is_empty() {
                out.push_str("Composite Hotspots (Churn × Blast Radius):\n");
                for h in &report.composite_hotspots {
                    out.push_str(&format!(
                        "  • {:<36} [Risk: {:>3}/100 | Churn: {:>2} commits | Fan-In: {:>2} mods]\n",
                        theme.bold(&h.path),
                        h.hotspot_risk_score,
                        h.commit_count,
                        h.dependent_modules_count
                    ));
                }
            }
            Ok(out)
        }
    }
}

/// Executes `eforge blame-arch` to attribute architectural violations to Git commits.
pub fn execute_blame_arch(
    repo_root: &Path,
    target_filter: Option<&str>,
) -> Result<Vec<ArchBlameEntry>, ForgeError> {
    let scan_opts = ScanOptions {
        repo_root: repo_root.to_path_buf(),
        incremental: true,
        format: OutputFormat::Json,
    };
    let (repo, _) = execute_scan(&scan_opts)?;
    let file_edges = extract_file_import_edges(repo_root, &repo.files)?;
    Ok(blame_architecture_violations(
        repo_root,
        &repo.findings,
        &file_edges,
        target_filter,
    ))
}

/// Renders [`ArchBlameEntry`] list in the requested [`OutputFormat`].
pub fn render_blame_arch_report(
    entries: &[ArchBlameEntry],
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(entries).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut md = String::new();
            md.push_str("# ELCODE Forge Architectural Blame Report\n\n");
            if entries.is_empty() {
                md.push_str("_No matching architectural violations found._\n");
            } else {
                md.push_str("| Rule | Location | Commit | Author | Summary |\n| :--- | :--- | :--- | :--- | :--- |\n");
                for e in entries {
                    md.push_str(&format!(
                        "| `{}` | `{}:{}` | `{}` | {} | {} |\n",
                        e.rule_id,
                        e.file_path,
                        e.line,
                        &e.commit_sha[..8.min(e.commit_sha.len())],
                        e.author,
                        e.commit_summary
                    ));
                }
            }
            Ok(md)
        }
        OutputFormat::Terminal => {
            let theme = Theme::default();
            let mut out = String::new();
            out.push_str("============================================================\n");
            out.push_str(&format!(
                " {}\n",
                theme.bold("ELCODE Forge Architectural Blame Attribution")
            ));
            out.push_str("============================================================\n");
            if entries.is_empty() {
                out.push_str("  No architectural violations matched.\n");
            } else {
                for e in entries {
                    let short_sha = &e.commit_sha[..8.min(e.commit_sha.len())];
                    out.push_str(&format!(
                        "  [{}] {}\n    Introduced by: {} ({} <{}>)\n    Commit:        {}\n    Evidence:      {}\n\n",
                        theme.bold(&e.rule_id),
                        theme.cyan(&format!("{}:{}", e.file_path, e.line)),
                        theme.yellow(short_sha),
                        e.author,
                        e.author_email,
                        e.commit_summary,
                        e.offending_code
                    ));
                }
            }
            Ok(out)
        }
    }
}
