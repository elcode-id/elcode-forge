//! `eforge fix` command — Deterministic surgical codemods and transactional rollback.

use std::path::Path;

use forge_core::ForgeError;
use forge_rules::{
    apply_fix_plan_transactional, generate_fix_plan, list_transactions, rollback_transaction,
    FixPlan, TransactionRecord,
};
use serde::{Deserialize, Serialize};

use crate::cli::OutputFormat;
use crate::commands::scan::{execute_scan, ScanOptions};
use crate::theme::Theme;

/// Result envelope for `eforge fix`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixCommandReport {
    pub mode: String,
    pub plan: Option<FixPlan>,
    pub transaction: Option<TransactionRecord>,
    pub transactions_list: Vec<TransactionRecord>,
}

/// Executes `eforge fix` in `--dry-run`, `--apply`, `--rollback`, or `--list-tx` mode.
pub fn execute_fix(
    repo_root: &Path,
    apply: bool,
    rule_filter: Option<&str>,
    rollback: Option<Option<&str>>,
    list_tx: bool,
) -> Result<FixCommandReport, ForgeError> {
    if list_tx {
        let txs = list_transactions(repo_root)?;
        return Ok(FixCommandReport {
            mode: "list_transactions".to_string(),
            plan: None,
            transaction: None,
            transactions_list: txs,
        });
    }

    if let Some(target_tx) = rollback {
        let restored = rollback_transaction(repo_root, target_tx)?;
        return Ok(FixCommandReport {
            mode: "rollback".to_string(),
            plan: None,
            transaction: Some(restored),
            transactions_list: Vec::new(),
        });
    }

    let scan_opts = ScanOptions {
        repo_root: repo_root.to_path_buf(),
        incremental: false,
        format: OutputFormat::Json,
    };
    let (repo, _) = execute_scan(&scan_opts)?;
    let plan = generate_fix_plan(repo_root, &repo, rule_filter)?;

    if apply {
        let tx = apply_fix_plan_transactional(repo_root, &plan.suggestions)?;
        Ok(FixCommandReport {
            mode: "applied".to_string(),
            plan: Some(plan),
            transaction: Some(tx),
            transactions_list: Vec::new(),
        })
    } else {
        Ok(FixCommandReport {
            mode: "dry_run".to_string(),
            plan: Some(plan),
            transaction: None,
            transactions_list: Vec::new(),
        })
    }
}

/// Renders a [`FixCommandReport`] in the requested [`OutputFormat`].
pub fn render_fix_report(
    report: &FixCommandReport,
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
            md.push_str(&format!(
                "# ELCODE Forge Auto-Fix Report (`{}`)\n\n",
                report.mode
            ));
            if let Some(plan) = &report.plan {
                md.push_str(&format!(
                    "- **Suggestions:** {}\n- **Total Edits:** {}\n- **Affected Files:** {}\n\n",
                    plan.suggestions.len(),
                    plan.total_edits,
                    plan.affected_files.len()
                ));
                for s in &plan.suggestions {
                    md.push_str(&format!(
                        "### `{}` — {}\n{}\n\n",
                        s.rule_id, s.title, s.description
                    ));
                }
            }
            if let Some(tx) = &report.transaction {
                md.push_str(&format!(
                    "- **Transaction ID:** `{}` ({} edit(s))\n",
                    tx.tx_id, tx.edits_applied
                ));
            }
            Ok(md)
        }
        OutputFormat::Terminal => {
            let theme = Theme::default();
            let mut out = String::new();
            out.push_str("============================================================\n");
            out.push_str(&format!(
                " {}\n",
                theme.bold(&format!(
                    "ELCODE Forge Surgical Codemods — mode: {}",
                    report.mode
                ))
            ));
            out.push_str("============================================================\n");

            if report.mode == "list_transactions" {
                if report.transactions_list.is_empty() {
                    out.push_str("  No recorded transactions in .eforge/tx/.\n");
                } else {
                    for tx in &report.transactions_list {
                        out.push_str(&format!(
                            "  • {} — {} edit(s) across {} file(s)\n",
                            theme.cyan(&tx.tx_id),
                            tx.edits_applied,
                            tx.files.len()
                        ));
                    }
                }
                return Ok(out);
            }

            if report.mode == "rollback" {
                if let Some(tx) = &report.transaction {
                    out.push_str(&format!(
                        "  {} Rolled back transaction {} ({} file(s) restored byte-for-byte).\n",
                        theme.green("✓"),
                        theme.cyan(&tx.tx_id),
                        tx.files.len()
                    ));
                }
                return Ok(out);
            }

            if let Some(plan) = &report.plan {
                out.push_str(&format!(
                    "Fix Suggestions: {} ({} surgical edit(s) across {} file(s))\n\n",
                    plan.suggestions.len(),
                    plan.total_edits,
                    plan.affected_files.len()
                ));
                for s in &plan.suggestions {
                    out.push_str(&format!(
                        "  [{}] {}\n",
                        theme.bold(&s.rule_id),
                        theme.cyan(&s.title)
                    ));
                    for edit in &s.edits {
                        out.push_str(&format!(
                            "    File: {}:{}..{}\n",
                            edit.file_path, edit.start_line, edit.end_line
                        ));
                        if !edit.original_text.is_empty() {
                            out.push_str(&format!(
                                "      {}\n",
                                theme.red(&format!("- {}", edit.original_text.trim()))
                            ));
                        }
                        if !edit.replacement_text.is_empty() {
                            let first = edit.replacement_text.lines().next().unwrap_or("");
                            out.push_str(&format!(
                                "      {}\n",
                                theme.green(&format!("+ {}", first.trim()))
                            ));
                        }
                    }
                    out.push('\n');
                }
            }

            if let Some(tx) = &report.transaction {
                out.push_str(&format!(
                    "Transaction Recorded: {} (use `eforge fix --rollback {}` to undo)\n",
                    theme.green(&tx.tx_id),
                    tx.tx_id
                ));
            } else {
                out.push_str(
                    "Dry-run complete (no files modified). Pass `--apply` to execute transactionally.\n",
                );
            }
            Ok(out)
        }
    }
}
