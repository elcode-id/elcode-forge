//! Tactile Terminal User Interface (`eforge ui` and `--interactive` on `doctor`/`scan`/`review`).
//!
//! Provides a 4-tab interactive terminal instrument for exploring the module DAG,
//! Tarjan SCC circular dependency rings, blast-radius radar, and code-frame findings deck.

use std::collections::BTreeSet;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use forge_analyzer::{
    analyze_architecture_incremental, detector::is_test_file, infer_topology_and_boundaries,
    trace_cycle_call_chains, FileAstCache, TopologyInferenceReport,
};
use forge_core::{CallChainHop, FileImportEdge, ForgeError, Repository};
use forge_git::{
    collect_working_tree_diff, compute_change_impact, compute_impacted_tests, parse_unified_diff,
    ChangeImpactReport, ImpactedTestsReport,
};
use serde::{Deserialize, Serialize};

use crate::cli::OutputFormat;
use crate::commands::scan::{execute_scan, ScanOptions};
use crate::formatters::codeframe::render_finding_codeframe;
use crate::theme::Theme;

/// Selectable tabs in the `eforge ui` interactive terminal instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TuiTab {
    /// Module DAG & Centrality Explorer
    #[default]
    #[value(name = "dag")]
    Dag,
    /// Tarjan SCC Circular Dependency Inspector
    #[value(name = "cycles")]
    Cycles,
    /// Working-Tree Blast Radius & Impacted Test Radar
    #[value(name = "blast")]
    Blast,
    /// Multi-Span Code-Frame Findings & Auto-Fix Deck
    #[value(name = "findings")]
    Findings,
}

impl TuiTab {
    /// Cycles to the next tab in order.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Dag => Self::Cycles,
            Self::Cycles => Self::Blast,
            Self::Blast => Self::Findings,
            Self::Findings => Self::Dag,
        }
    }
}

/// Pure, deterministic state machine for the ELCODE Forge TUI instrument.
#[derive(Debug, Clone)]
pub struct TuiAppState {
    pub repo_root: PathBuf,
    pub repo: Repository,
    pub cycles: Vec<Vec<String>>,
    pub file_edges: Vec<FileImportEdge>,
    pub cycle_traces: Vec<Vec<CallChainHop>>,
    pub topology: TopologyInferenceReport,
    pub impact: ChangeImpactReport,
    pub impacted_tests: ImpactedTestsReport,
    pub ast_cache_hits: usize,
    pub ast_cache_misses: usize,
    pub active_tab: TuiTab,
    pub selected_index: usize,
    pub filter_query: String,
}

impl TuiAppState {
    /// Builds a complete [`TuiAppState`] for `repo_root`.
    pub fn load(repo_root: &Path, initial_tab: TuiTab) -> Result<Self, ForgeError> {
        let scan_opts = ScanOptions {
            repo_root: repo_root.to_path_buf(),
            incremental: true,
            format: OutputFormat::Json,
        };
        let (repo, _) = execute_scan(&scan_opts)?;

        let mut ast_cache = FileAstCache::load(repo_root);
        let inc = analyze_architecture_incremental(repo_root, &repo.files, &mut ast_cache)?;
        let _ = ast_cache.save(repo_root);

        let cycle_traces =
            trace_cycle_call_chains(&inc.graph.circular_dependencies, &inc.file_edges);
        let topology = infer_topology_and_boundaries(&repo.files, &inc.graph);

        let diff_text = collect_working_tree_diff(repo_root).unwrap_or_default();
        let changes = parse_unified_diff(&diff_text).unwrap_or_default();
        let test_files: BTreeSet<String> = repo
            .files
            .iter()
            .filter(|f| is_test_file(&f.relative_path))
            .map(|f| f.relative_path.clone())
            .collect();

        let impact = compute_change_impact(&changes, &repo.modules, &test_files);
        let impacted_tests = compute_impacted_tests(
            &changes,
            &repo.modules,
            &inc.file_edges,
            &test_files,
            &repo.detected_languages,
            &repo.package_managers,
        );

        Ok(Self {
            repo_root: repo_root.to_path_buf(),
            repo,
            cycles: inc.graph.circular_dependencies,
            file_edges: inc.file_edges,
            cycle_traces,
            topology,
            impact,
            impacted_tests,
            ast_cache_hits: inc.cache_hits,
            ast_cache_misses: inc.cache_misses,
            active_tab: initial_tab,
            selected_index: 0,
            filter_query: String::new(),
        })
    }

    /// Moves selection down within the active tab's list.
    pub fn next_item(&mut self) {
        let count = self.active_list_len();
        if count > 0 {
            self.selected_index = (self.selected_index + 1) % count;
        }
    }

    /// Moves selection up within the active tab's list.
    pub fn prev_item(&mut self) {
        let count = self.active_list_len();
        if count > 0 {
            self.selected_index = if self.selected_index == 0 {
                count - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    /// Switches to `tab` and resets `selected_index` to `0`.
    pub fn select_tab(&mut self, tab: TuiTab) {
        self.active_tab = tab;
        self.selected_index = 0;
    }

    fn active_list_len(&self) -> usize {
        match self.active_tab {
            TuiTab::Dag => self.topology.module_metrics.len(),
            TuiTab::Cycles => self.cycles.len(),
            TuiTab::Blast => self.impact.changed_files.len(),
            TuiTab::Findings => self.repo.findings.len(),
        }
    }
}

/// Renders the current [`TuiAppState`] into a tactile 3-column terminal frame.
#[must_use]
pub fn render_tui_frame(state: &TuiAppState, theme: &Theme) -> String {
    let mut out = String::new();
    let health_score = state.repo.health.as_ref().map_or(100, |h| h.overall);
    let meter = theme.render_score_meter(health_score);

    let top_bar = if theme.unicode_enabled {
        "┌─────────────────────────────────────────────────────────────────────────────────────────────┐"
    } else {
        "+---------------------------------------------------------------------------------------------+"
    };
    let mid_bar = if theme.unicode_enabled {
        "├─────────────────────────────────────────────────────────────────────────────────────────────┤"
    } else {
        "+---------------------------------------------------------------------------------------------+"
    };
    let bot_bar = if theme.unicode_enabled {
        "└─────────────────────────────────────────────────────────────────────────────────────────────┘"
    } else {
        "+---------------------------------------------------------------------------------------------+"
    };

    out.push_str(&format!("{top_bar}\n"));
    out.push_str(&format!(
        "│ {} ── repo: {} ── Health: {}\n",
        theme.bold("ELCODE Forge Instrument v1.1"),
        theme.cyan(&state.repo.name),
        meter
    ));

    let tab_dag = format_tab_label("1: DAG Explorer", state.active_tab == TuiTab::Dag, theme);
    let tab_cyc = format_tab_label(
        &format!("2: SCC Cycles ({})", state.cycles.len()),
        state.active_tab == TuiTab::Cycles,
        theme,
    );
    let tab_blast = format_tab_label(
        &format!(
            "3: Blast Radar ({} files)",
            state.impact.changed_files.len()
        ),
        state.active_tab == TuiTab::Blast,
        theme,
    );
    let tab_find = format_tab_label(
        &format!("4: Findings Deck ({})", state.repo.findings.len()),
        state.active_tab == TuiTab::Findings,
        theme,
    );
    out.push_str(&format!(
        "│ {tab_dag}   {tab_cyc}   {tab_blast}   {tab_find}\n"
    ));
    out.push_str(&format!("{mid_bar}\n"));

    match state.active_tab {
        TuiTab::Dag => render_dag_tab(state, theme, &mut out),
        TuiTab::Cycles => render_cycles_tab(state, theme, &mut out),
        TuiTab::Blast => render_blast_tab(state, theme, &mut out),
        TuiTab::Findings => render_findings_tab(state, theme, &mut out),
    }

    out.push_str(&format!("{mid_bar}\n"));
    out.push_str("│ [j/k] Navigate   [1-4/Tab] Switch View   [/] Filter   [q] Quit Instrument\n");
    out.push_str(&format!("{bot_bar}\n"));
    out
}

fn format_tab_label(label: &str, active: bool, theme: &Theme) -> String {
    if active {
        theme.bold(&theme.cyan(&format!("[{label}]")))
    } else {
        theme.dim(label)
    }
}

fn render_dag_tab(state: &TuiAppState, theme: &Theme, out: &mut String) {
    out.push_str(&format!(
        "│ Archetype: {}   │ BLAKE3 AST Cache: {} hit / {} miss\n",
        theme.bold(&state.topology.archetype_label),
        state.ast_cache_hits,
        state.ast_cache_misses
    ));
    out.push_str("│\n");

    if state.topology.module_metrics.is_empty() {
        out.push_str("│   (No source modules discovered in repository)\n");
        return;
    }

    let sel_idx = state
        .selected_index
        .min(state.topology.module_metrics.len().saturating_sub(1));
    let selected_metric = &state.topology.module_metrics[sel_idx];
    let selected_mod = state
        .repo
        .modules
        .iter()
        .find(|m| m.id == selected_metric.module_id);

    out.push_str("│ MODULES (Ranked by Centrality Ca):\n");
    for (idx, m) in state.topology.module_metrics.iter().enumerate() {
        let cursor = if idx == sel_idx { ">" } else { " " };
        let line = format!(
            "│  {cursor} {:<24} [Ca: {:>2} | Ce: {:>2} | I: {:>4.2} | Layer: {}]",
            m.module_id,
            m.afferent_coupling_ca,
            m.efferent_coupling_ce,
            m.instability_index,
            m.inferred_layer.as_str()
        );
        if idx == sel_idx {
            out.push_str(&format!("{}\n", theme.bold(&line)));
        } else {
            out.push_str(&format!("{line}\n"));
        }
    }

    if let Some(m) = selected_mod {
        let inbound: Vec<&str> = state
            .repo
            .modules
            .iter()
            .filter(|other| other.imports.contains(&m.id))
            .map(|other| other.id.as_str())
            .collect();

        out.push_str("│\n");
        out.push_str(&format!(
            "│ TOPOLOGY FOCUS: {} ({} file(s))\n",
            theme.bold(&m.id),
            m.files.len()
        ));
        out.push_str(&format!(
            "│   Inbound Consumers ({}): {}\n",
            inbound.len(),
            if inbound.is_empty() {
                "none (top-level entrypoint)".to_string()
            } else {
                inbound.join(", ")
            }
        ));
        out.push_str(&format!(
            "│   Outbound Imports  ({}): {}\n",
            m.imports.len(),
            if m.imports.is_empty() {
                "none (foundational leaf module)".to_string()
            } else {
                m.imports.join(", ")
            }
        ));
    }
}

fn render_cycles_tab(state: &TuiAppState, theme: &Theme, out: &mut String) {
    if state.cycles.is_empty() {
        out.push_str(&format!(
            "│  {}\n",
            theme.green(
                "✓ Architecture graph is a strict Directed Acyclic Graph (0 Tarjan SCC cycles)."
            )
        ));
        return;
    }

    let sel_idx = state
        .selected_index
        .min(state.cycles.len().saturating_sub(1));
    for (idx, cycle) in state.cycles.iter().enumerate() {
        let cursor = if idx == sel_idx { ">" } else { " " };
        let mut ring = cycle.clone();
        if let Some(first) = cycle.first() {
            ring.push(first.clone());
        }
        out.push_str(&format!(
            "│ {cursor} SCC Cycle #{}: {}\n",
            idx + 1,
            theme.red(&ring.join(" ──▶ "))
        ));
    }

    if let Some(hops) = state.cycle_traces.get(sel_idx) {
        out.push_str("│\n│ Call-Chain Edge Trace & Feedback Arc Candidates:\n");
        for (idx, hop) in hops.iter().enumerate() {
            let badge = if hop.is_feedback_arc_candidate {
                " ★ Minimum Feedback Arc Break Candidate"
            } else {
                ""
            };
            out.push_str(&format!(
                "│   [Hop {}] {}:{}:{} ──imports '{}'──▶ {}{}\n",
                idx + 1,
                hop.from_file,
                hop.line,
                hop.column,
                hop.raw_specifier,
                hop.to_file,
                theme.yellow(badge)
            ));
        }
    }
}

fn render_blast_tab(state: &TuiAppState, theme: &Theme, out: &mut String) {
    out.push_str(&format!(
        "│ Working-Tree Changes: {} file(s) │ Direct Modules: {} │ Transitive Modules: {}\n",
        state.impact.changed_files.len(),
        state.impact.directly_affected_modules.len(),
        state.impact.transitively_affected_modules.len()
    ));
    out.push_str(&format!(
        "│ Direct Blast Radius:     {}\n",
        if state.impact.directly_affected_modules.is_empty() {
            "none".to_string()
        } else {
            state.impact.directly_affected_modules.join(", ")
        }
    ));
    out.push_str(&format!(
        "│ Transitive Blast Radius: {}\n",
        if state.impact.transitively_affected_modules.is_empty() {
            "none".to_string()
        } else {
            state.impact.transitively_affected_modules.join(", ")
        }
    ));
    out.push_str("│\n│ SMART IMPACTED TEST SELECTION:\n");
    out.push_str(&format!(
        "│   Selected Tests ({}): {}\n",
        state.impacted_tests.all_selected_tests.len(),
        if state.impacted_tests.all_selected_tests.is_empty() {
            "none".to_string()
        } else {
            state.impacted_tests.all_selected_tests.join(", ")
        }
    ));
    for cmd in &state.impacted_tests.recommended_commands {
        out.push_str(&format!("│   Runner Command: {}\n", theme.cyan(cmd)));
    }
}

fn render_findings_tab(state: &TuiAppState, theme: &Theme, out: &mut String) {
    if state.repo.findings.is_empty() {
        out.push_str("│  ✓ Zero findings detected across repository.\n");
        return;
    }

    let sel_idx = state
        .selected_index
        .min(state.repo.findings.len().saturating_sub(1));
    let selected = &state.repo.findings[sel_idx];

    out.push_str(&format!(
        "│ Showing Finding {}/{} (use j/k to cycle findings):\n│\n",
        sel_idx + 1,
        state.repo.findings.len()
    ));

    let card = render_finding_codeframe(
        &state.repo_root,
        &state.repo,
        selected,
        &state.file_edges,
        &state.cycle_traces,
        theme,
    );
    for line in card.lines() {
        out.push_str(&format!("│ {line}\n"));
    }
}

/// Launches the `eforge ui` tactile terminal instrument.
///
/// When `once` is `true` or `stdin`/`stdout` is not an interactive terminal, renders a single
/// deterministic snapshot frame and returns immediately.
pub fn run_tui(repo_root: &Path, initial_tab: TuiTab, once: bool) -> Result<String, ForgeError> {
    let mut state = TuiAppState::load(repo_root, initial_tab)?;
    let theme = Theme::default();

    if once || !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(render_tui_frame(&state, &theme));
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // Clear screen and move cursor to top-left
        let frame = render_tui_frame(&state, &theme);
        write!(stdout, "\x1b[2J\x1b[H{frame}").map_err(|source| ForgeError::Io {
            path: repo_root.to_path_buf(),
            source,
        })?;
        write!(
            stdout,
            "Command [j=down, k=up, 1-4=tab, tab=next-tab, q=quit] > "
        )
        .map_err(|source| ForgeError::Io {
            path: repo_root.to_path_buf(),
            source,
        })?;
        stdout.flush().map_err(|source| ForgeError::Io {
            path: repo_root.to_path_buf(),
            source,
        })?;

        let mut line = String::new();
        let bytes = stdin
            .lock()
            .read_line(&mut line)
            .map_err(|source| ForgeError::Io {
                path: repo_root.to_path_buf(),
                source,
            })?;
        if bytes == 0 {
            break;
        }
        let cmd = line.trim();
        match cmd {
            "q" | "quit" | "exit" => break,
            "j" | "down" | "n" => state.next_item(),
            "k" | "up" | "p" => state.prev_item(),
            "1" | "dag" => state.select_tab(TuiTab::Dag),
            "2" | "cycles" => state.select_tab(TuiTab::Cycles),
            "3" | "blast" => state.select_tab(TuiTab::Blast),
            "4" | "findings" => state.select_tab(TuiTab::Findings),
            "tab" | "t" | "" => state.select_tab(state.active_tab.next()),
            _ => {}
        }
    }

    Ok(render_tui_frame(&state, &theme))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_state_loads_and_renders_all_four_tabs_deterministically() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let state_res = TuiAppState::load(&fixture, TuiTab::Dag);
        assert!(state_res.is_ok());
        let Ok(mut state) = state_res else { return };

        let plain = Theme::plain();
        let dag_frame = render_tui_frame(&state, &plain);
        assert!(dag_frame.contains("ELCODE Forge Instrument v1.1"));
        assert!(dag_frame.contains("MODULES (Ranked by Centrality Ca)"));
        assert!(dag_frame.contains("src/db"));

        state.next_item();
        assert_eq!(state.selected_index, 1);
        state.prev_item();
        assert_eq!(state.selected_index, 0);

        state.select_tab(TuiTab::Cycles);
        let cyc_frame = render_tui_frame(&state, &plain);
        assert!(cyc_frame.contains("SCC Cycle #1"));
        assert!(cyc_frame.contains("Minimum Feedback Arc Break Candidate"));

        state.select_tab(TuiTab::Blast);
        let blast_frame = render_tui_frame(&state, &plain);
        assert!(blast_frame.contains("SMART IMPACTED TEST SELECTION"));

        state.select_tab(TuiTab::Findings);
        let find_frame = render_tui_frame(&state, &plain);
        assert!(find_frame.contains("Showing Finding 1/"));
    }
}
