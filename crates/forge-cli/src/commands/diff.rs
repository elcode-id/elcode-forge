//! `eforge diff` command — working-tree unified diff and blast-radius impact inspection.

use std::collections::BTreeSet;
use std::path::Path;

use forge_analyzer::{build_polyglot_architecture_graph, scanner::scan_repository};
use forge_core::scoring::calculate_health_score;
use forge_core::sync::DeltaSummary;
use forge_core::{config::load_config, ForgeError};
use forge_git::{
    collect_working_tree_diff, compute_change_impact, parse_unified_diff, ChangeImpactReport,
};

use crate::cli::OutputFormat;
use crate::commands::scan::{execute_scan, ScanOptions};
use crate::commands::sync::load_cached_baseline;
use crate::exit_code::ExitCode;

/// Computes the [`ChangeImpactReport`] for `repo_root` using either an explicit unified `diff_input`
/// or the live Git working-tree diff (`git diff HEAD`).
pub fn execute_diff(
    repo_root: &Path,
    diff_input: Option<&str>,
) -> Result<ChangeImpactReport, ForgeError> {
    let config = load_config(repo_root)?;
    let discovery = scan_repository(repo_root, &config.ignore_patterns)?;
    let arch_graph = build_polyglot_architecture_graph(repo_root, &discovery.files)?;

    let raw_diff = match diff_input {
        Some(text) => text.to_string(),
        None => collect_working_tree_diff(repo_root).unwrap_or_default(),
    };

    let changes = parse_unified_diff(&raw_diff)?;
    let test_files: BTreeSet<String> = discovery
        .files
        .iter()
        .filter(|f| f.is_test)
        .map(|f| f.relative_path.clone())
        .collect();

    Ok(compute_change_impact(
        &changes,
        &arch_graph.modules,
        &test_files,
    ))
}

/// Renders a [`ChangeImpactReport`] in the requested [`OutputFormat`].
pub fn render_diff_report(
    report: &ChangeImpactReport,
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
            md.push_str("# ELCODE Forge Diff Impact\n\n");
            md.push_str(&format!(
                "- **Changed Files:** {}\n- **Directly Affected Modules:** `{}`\n- **Transitively Affected Modules:** `{}`\n- **Changed Files Missing Tests:** {}\n",
                report.changed_files.len(),
                if report.directly_affected_modules.is_empty() {
                    "none".to_string()
                } else {
                    report.directly_affected_modules.join(", ")
                },
                if report.transitively_affected_modules.is_empty() {
                    "none".to_string()
                } else {
                    report.transitively_affected_modules.join(", ")
                },
                report.changed_files_without_tests.len()
            ));
            Ok(md)
        }
        OutputFormat::Terminal => {
            let mut out = String::new();
            out.push_str("ELCODE Forge Diff Impact\n");
            out.push_str("------------------------\n");
            out.push_str(&format!("Changed files: {}\n", report.changed_files.len()));
            for file in &report.changed_files {
                out.push_str(&format!(
                    "  - {} (+{} / -{})\n",
                    file.path, file.added_lines, file.deleted_lines
                ));
            }
            out.push_str(&format!(
                "Directly affected modules: {}\n",
                if report.directly_affected_modules.is_empty() {
                    "none".to_string()
                } else {
                    report.directly_affected_modules.join(", ")
                }
            ));
            out.push_str(&format!(
                "Transitively affected modules: {}\n",
                if report.transitively_affected_modules.is_empty() {
                    "none".to_string()
                } else {
                    report.transitively_affected_modules.join(", ")
                }
            ));
            Ok(out)
        }
    }
}

/// Executes remote diff comparing local scan against the cached or pulled remote baseline snapshot.
pub fn run_diff_remote(repo_root: &Path, format: OutputFormat) -> ExitCode {
    let baseline_opt = match load_cached_baseline(repo_root) {
        Ok(Some(base)) => Some(base),
        _ => {
            eprintln!("Notice: No local baseline cached at .eforge/baseline.json.");
            eprintln!("Run 'eforge pull' first to fetch baseline from remote server.");
            return ExitCode::ConfigurationError;
        }
    };

    let Some(baseline) = baseline_opt else {
        return ExitCode::ConfigurationError;
    };

    let scan_opts = ScanOptions {
        repo_root: repo_root.to_path_buf(),
        incremental: false,
        format: OutputFormat::Json,
    };

    let (repo_ir, _) = match execute_scan(&scan_opts) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error scanning repository for diff: {e}");
            return ExitCode::ExecutionError;
        }
    };

    let health_score = repo_ir
        .health
        .clone()
        .unwrap_or_else(|| calculate_health_score(&repo_ir));

    let delta = DeltaSummary::compute(
        &repo_ir.findings,
        &baseline.baseline_findings,
        &health_score,
        baseline.baseline_health_score.as_ref(),
    );

    let baseline_sha = baseline
        .baseline_commit_hash
        .as_deref()
        .unwrap_or("unknown");
    let current_sha = forge_git::log::read_head_revision(repo_root)
        .ok()
        .flatten()
        .map(|r| r.head_sha)
        .unwrap_or_else(|| "working-tree".to_string());

    if format == OutputFormat::Json {
        let json_val = serde_json::json!({
            "baseline_commit": baseline_sha,
            "current_commit": current_sha,
            "baseline_health": baseline.baseline_health_score.map(|h| h.overall),
            "current_health": health_score.overall,
            "delta": delta,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&json_val).unwrap_or_default()
        );
        return ExitCode::Success;
    }

    println!("ELCODE Forge Remote Diff Comparison");
    println!("-----------------------------------");
    println!("  Baseline Commit: {baseline_sha}");
    println!("  Current Commit:  {current_sha}");
    let base_overall = baseline
        .baseline_health_score
        .map(|h| h.overall as i16)
        .unwrap_or(health_score.overall as i16);
    println!(
        "  Health Score:    {}/100 (baseline: {}, delta: {:+})",
        health_score.overall, base_overall, delta.health_score_delta
    );
    println!(
        "  Findings:        +{} added, -{} resolved, {} unchanged",
        delta.added_findings, delta.resolved_findings, delta.unchanged_findings
    );

    let added_findings: Vec<_> = repo_ir
        .findings
        .iter()
        .filter(|f| !baseline.baseline_findings.iter().any(|b| b.id == f.id))
        .collect();

    if !added_findings.is_empty() {
        println!("\n  Newly Introduced Findings:");
        for f in added_findings {
            let loc = f
                .location
                .as_ref()
                .map(|l| format!(" ({}:{})", l.file_path, l.start_line.unwrap_or(1)))
                .unwrap_or_default();
            println!("    - [{:?}] {}{}", f.severity, f.title, loc);
        }
    }

    ExitCode::Success
}
