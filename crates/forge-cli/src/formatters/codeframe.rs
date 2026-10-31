//! Rich `miette`/`ariadne`-inspired multi-span code-frame diagnostic formatter for `eforge`.
//!
//! Renders source file gutters, column caret underlines (`^~~~`), Tarjan SCC cycle call-chain
//! arrows (`├─▶ [Hop N]`), and inline surgical auto-fix diff previews (`-` / `+`) with full
//! support for `--no-color` and ASCII fallback modes.

use std::fs;
use std::path::Path;

use forge_analyzer::{extract_file_import_edges, trace_cycle_call_chains};
use forge_core::path_guard::resolve_safe_path;
use forge_core::{CallChainHop, FileImportEdge, Finding, Repository};
use forge_rules::build_suggestion_for_finding;

use crate::theme::Theme;

/// Renders a full [`Repository`] report in rich code-frame mode (`--codeframe`).
#[must_use]
pub fn render_codeframe_report(repo_root: &Path, repo: &Repository, theme: &Theme) -> String {
    let file_edges = extract_file_import_edges(repo_root, &repo.files).unwrap_or_default();
    let mut cycles = Vec::new();
    // Reconstruct cycle list from findings or module graph if present
    for f in &repo.findings {
        if f.rule_id == "arch/no-circular-dependencies" {
            let mods: Vec<String> = repo
                .modules
                .iter()
                .filter(|m| f.evidence.contains(&m.id) || f.description.contains(&m.id))
                .map(|m| m.id.clone())
                .collect();
            if mods.len() >= 2 {
                cycles.push(mods);
            }
        }
    }
    let cycle_traces = trace_cycle_call_chains(&cycles, &file_edges);

    let mut out = String::new();
    out.push_str("============================================================\n");
    out.push_str(&format!(
        " {}\n",
        theme.bold(&format!(
            "ELCODE Forge Diagnostic Instrument — {}",
            repo.name
        ))
    ));
    out.push_str("============================================================\n");
    if let Some(health) = &repo.health {
        out.push_str(&format!(
            "Overall Health: {}\n\n",
            theme.render_score_meter(health.overall)
        ));
    }

    if repo.findings.is_empty() {
        out.push_str("  ✓ Zero architectural, quality, dependency, or security findings.\n");
        return out;
    }

    out.push_str(&format!(
        "Code-Frame Diagnostics ({}):\n\n",
        repo.findings.len()
    ));
    for finding in &repo.findings {
        out.push_str(&render_finding_codeframe(
            repo_root,
            repo,
            finding,
            &file_edges,
            &cycle_traces,
            theme,
        ));
        out.push('\n');
    }

    out
}

/// Renders a single [`Finding`] as a rich multi-line code-frame card with source gutter,
/// caret highlight, call-chain trace, and surgical fix preview.
#[must_use]
pub fn render_finding_codeframe(
    repo_root: &Path,
    repo: &Repository,
    finding: &Finding,
    file_edges: &[FileImportEdge],
    cycle_traces: &[Vec<CallChainHop>],
    theme: &Theme,
) -> String {
    let mut out = String::new();
    let badge = theme.badge_severity(finding.severity);
    let dash = if theme.unicode_enabled {
        "──"
    } else {
        "--"
    };
    let top_corner = if theme.unicode_enabled {
        "┌─▶"
    } else {
        "+->"
    };
    let vbar = if theme.unicode_enabled { "│" } else { "|" };
    let mid_branch = if theme.unicode_enabled {
        "├─●"
    } else {
        "|--"
    };
    let fix_branch = if theme.unicode_enabled {
        "├─✦"
    } else {
        "|**"
    };
    let bot_corner = if theme.unicode_enabled {
        "└─▶"
    } else {
        "+->"
    };

    out.push_str(&format!(
        "  {} {} {} {}\n",
        badge,
        theme.bold(&finding.rule_id),
        dash,
        theme.bold(&finding.title)
    ));

    if let Some(loc) = &finding.location {
        let target_line = loc.start_line.unwrap_or(1);
        let matched_edge = file_edges
            .iter()
            .find(|e| e.from_file == loc.file_path && (e.line == target_line || target_line == 1));
        let display_line = matched_edge.map_or(target_line, |e| e.line);
        let display_col = matched_edge.map_or(1, |e| e.column);

        out.push_str(&format!(
            "  {} {}\n",
            top_corner,
            theme.cyan(&format!(
                "{}:{}:{}",
                loc.file_path, display_line, display_col
            ))
        ));

        if let Some(frame) =
            read_source_window(repo_root, &loc.file_path, display_line, matched_edge, theme)
        {
            out.push_str(&format!("  {vbar}\n"));
            out.push_str(&frame);
        }

        if let Some(edge) = matched_edge {
            let arrow = if theme.unicode_enabled {
                "──imports──▶"
            } else {
                "--imports-->"
            };
            out.push_str(&format!("  {mid_branch} Call-Chain Context:\n"));
            out.push_str(&format!(
                "  {vbar}   {}:{} {} {} (module '{}' -> '{}')\n",
                edge.from_file, edge.line, arrow, edge.to_file, edge.from_module, edge.to_module
            ));
        }
    } else if finding.rule_id == "arch/no-circular-dependencies" {
        out.push_str(&format!(
            "  {} {}\n",
            top_corner,
            theme.cyan(&finding.evidence)
        ));
        if let Some(hops) = cycle_traces.first() {
            for (idx, hop) in hops.iter().enumerate() {
                let marker = if hop.is_feedback_arc_candidate {
                    " ★ Minimum Feedback Arc Candidate"
                } else {
                    ""
                };
                out.push_str(&format!(
                    "  {mid_branch} [Hop {}] {}:{}:{} -> imports '{}' ({}){}\n",
                    idx + 1,
                    hop.from_file,
                    hop.line,
                    hop.column,
                    hop.raw_specifier,
                    hop.to_module,
                    theme.yellow(marker)
                ));
            }
        }
    }

    if let Some(suggestion) = build_suggestion_for_finding(repo_root, repo, finding) {
        out.push_str(&format!(
            "  {fix_branch} Suggested Surgical Fix (`eforge fix --rule {} --dry-run`):\n",
            finding.rule_id
        ));
        for edit in &suggestion.edits {
            if !edit.original_text.is_empty() {
                out.push_str(&format!(
                    "  {vbar} {}\n",
                    theme.red(&format!("- {}", edit.original_text.trim()))
                ));
            }
            if !edit.replacement_text.is_empty() {
                let first_line = edit.replacement_text.lines().next().unwrap_or("");
                out.push_str(&format!(
                    "  {vbar} {}\n",
                    theme.green(&format!("+ {}", first_line.trim()))
                ));
            }
        }
    }

    out.push_str(&format!(
        "  {} Recommendation: {}\n",
        bot_corner, finding.recommendation
    ));
    out
}

fn read_source_window(
    repo_root: &Path,
    rel_path: &str,
    center_line: u32,
    matched_edge: Option<&FileImportEdge>,
    theme: &Theme,
) -> Option<String> {
    let safe_path = resolve_safe_path(repo_root, Path::new(rel_path)).ok()?;
    let content = fs::read_to_string(safe_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return None;
    }

    let total = lines.len() as u32;
    let target = center_line.clamp(1, total);
    let start = target.saturating_sub(2).max(1);
    let end = (target + 1).min(total);
    let vbar = if theme.unicode_enabled { "│" } else { "|" };

    let mut out = String::new();
    for line_no in start..=end {
        let text = lines.get((line_no - 1) as usize).copied().unwrap_or("");
        if line_no == target {
            out.push_str(&format!(
                "{:>4} {} {}\n",
                theme.bold(&line_no.to_string()),
                vbar,
                text
            ));
            let (col_start, span_len) = compute_caret_span(text, matched_edge);
            let pad = " ".repeat(col_start.saturating_sub(1));
            let carets = "^".repeat(span_len.max(1));
            out.push_str(&format!("     {} {}{}\n", vbar, pad, theme.red(&carets)));
        } else {
            out.push_str(&format!("{:>4} {} {}\n", line_no, vbar, theme.dim(text)));
        }
    }
    Some(out)
}

fn compute_caret_span(line_text: &str, matched_edge: Option<&FileImportEdge>) -> (usize, usize) {
    if let Some(edge) = matched_edge {
        if let Some(pos) = line_text.find(&edge.raw_specifier) {
            let col = line_text[..pos].chars().count() + 1;
            let len = edge.raw_specifier.chars().count();
            return (col, len);
        }
    }
    let trimmed = line_text.trim_start();
    let leading = line_text
        .chars()
        .count()
        .saturating_sub(trimmed.chars().count());
    (leading + 1, trimmed.chars().count().min(48))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::OutputFormat;
    use crate::commands::scan::{execute_scan, ScanOptions};
    use std::path::PathBuf;

    #[test]
    fn renders_codeframe_with_source_gutter_and_fix_preview_on_next_app() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let opts = ScanOptions {
            repo_root: fixture.clone(),
            incremental: false,
            format: OutputFormat::Terminal,
        };
        let scan_res = execute_scan(&opts);
        assert!(scan_res.is_ok());
        let Ok((repo, _)) = scan_res else { return };

        let plain = Theme::plain();
        let output = render_codeframe_report(&fixture, &repo, &plain);
        assert!(output.contains("ELCODE Forge Diagnostic Instrument"));
        assert!(output.contains("arch/no-direct-db-in-ui"));
        assert!(output.contains("Suggested Surgical Fix"));
        assert!(output.contains("^"));
    }
}
