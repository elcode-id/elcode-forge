//! `eforge simulate` command — In-memory refactoring and topology simulator.

use std::fs;
use std::path::Path;

use forge_analyzer::{
    parse_refactor_plan, scanner::scan_repository, simulate_refactor, RefactorOperation,
    SimulationReport,
};
use forge_core::{config::load_config, path_guard::resolve_safe_path, ForgeError};

use crate::cli::OutputFormat;
use crate::theme::Theme;

/// Executes `eforge simulate` for either an explicit `mv <from> <to>` pair or a `--plan <file.yaml>`.
pub fn execute_simulate(
    repo_root: &Path,
    move_pair: Option<(&str, &str)>,
    plan_path: Option<&Path>,
) -> Result<SimulationReport, ForgeError> {
    let config = load_config(repo_root)?;
    let discovery = scan_repository(repo_root, &config.ignore_patterns)?;

    let mut operations = Vec::new();
    if let Some((from, to)) = move_pair {
        let is_file = discovery.files.iter().any(|f| f.relative_path == from);
        if is_file {
            operations.push(RefactorOperation::MoveFile {
                from: from.to_string(),
                to: to.to_string(),
            });
        } else {
            operations.push(RefactorOperation::MoveModule {
                from_prefix: from.to_string(),
                to_prefix: to.to_string(),
            });
        }
    }

    if let Some(plan_rel) = plan_path {
        let safe_plan = resolve_safe_path(repo_root, plan_rel)?;
        let content = fs::read_to_string(&safe_plan).map_err(|source| ForgeError::Io {
            path: safe_plan,
            source,
        })?;
        let parsed = parse_refactor_plan(&content)?;
        operations.extend(parsed.operations);
    }

    if operations.is_empty() {
        return Err(ForgeError::Config {
            message: "Must specify either `eforge simulate mv <from> <to>` or `--plan <file.yaml>`"
                .to_string(),
        });
    }

    simulate_refactor(repo_root, &discovery.files, &operations)
}

/// Renders a [`SimulationReport`] in the requested [`OutputFormat`].
pub fn render_simulation_report(
    report: &SimulationReport,
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
            md.push_str("# ELCODE Forge In-Memory Refactoring Simulation\n\n");
            md.push_str(&format!("- **Summary:** {}\n", report.summary));
            md.push_str(&format!(
                "- **Safe to Apply:** `{}`\n- **Resolved Cycles:** {}\n- **Introduced Cycles:** {}\n- **Import Rewrites Required:** {}\n",
                report.is_safe_to_apply,
                report.resolved_cycles.len(),
                report.introduced_cycles.len(),
                report.import_rewrites.len()
            ));
            Ok(md)
        }
        OutputFormat::Terminal => {
            let theme = Theme::default();
            let mut out = String::new();
            out.push_str("============================================================\n");
            out.push_str(&format!(
                " {}\n",
                theme.bold("ELCODE Forge In-Memory Refactoring Simulator")
            ));
            out.push_str("============================================================\n");
            out.push_str(&format!("Summary: {}\n", report.summary));
            let verdict = if report.is_safe_to_apply {
                theme.green("SAFE TO APPLY (0 new circular dependencies introduced)")
            } else {
                theme.red("UNSAFE (introduces new circular dependencies!)")
            };
            out.push_str(&format!("Verdict: {verdict}\n\n"));

            out.push_str(&format!(
                "Resolved Cycles ({}):\n",
                report.resolved_cycles.len()
            ));
            for c in &report.resolved_cycles {
                out.push_str(&format!("  {} {}\n", theme.green("✓"), c.join(" <-> ")));
            }

            out.push_str(&format!(
                "\nIntroduced Cycles ({}):\n",
                report.introduced_cycles.len()
            ));
            for c in &report.introduced_cycles {
                out.push_str(&format!("  {} {}\n", theme.red("✗"), c.join(" <-> ")));
            }

            out.push_str(&format!(
                "\nRequired Import Specifier Rewrites ({}):\n",
                report.import_rewrites.len()
            ));
            for rw in &report.import_rewrites {
                out.push_str(&format!(
                    "  • {}:{} : '{}' -> '{}'\n",
                    theme.cyan(&rw.importer_file),
                    rw.line,
                    theme.red(&rw.old_specifier),
                    theme.green(&rw.new_specifier)
                ));
            }
            Ok(out)
        }
    }
}
