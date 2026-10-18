//! `eforge diff` command — working-tree unified diff and blast-radius impact inspection.

use std::collections::BTreeSet;
use std::path::Path;

use forge_analyzer::{graph::build_ts_architecture_graph, scanner::scan_repository};
use forge_core::{config::load_config, ForgeError};
use forge_git::{
    collect_working_tree_diff, compute_change_impact, parse_unified_diff, ChangeImpactReport,
};

use crate::cli::OutputFormat;

/// Computes the [`ChangeImpactReport`] for `repo_root` using either an explicit unified `diff_input`
/// or the live Git working-tree diff (`git diff HEAD`).
pub fn execute_diff(
    repo_root: &Path,
    diff_input: Option<&str>,
) -> Result<ChangeImpactReport, ForgeError> {
    let config = load_config(repo_root)?;
    let discovery = scan_repository(repo_root, &config.ignore_patterns)?;
    let arch_graph = build_ts_architecture_graph(repo_root, &discovery.files)?;

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
