//! `eforge watch` command — Live `<50ms` incremental architectural radar.
//!
//! Uses [`FileAstCache`] keyed by per-file `blake3` content hashes so only modified files
//! are re-parsed on save, providing instant feedback on health score deltas, Tarjan SCC
//! cycles, and rule violations.

use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use forge_analyzer::{
    analyze_architecture_incremental, analyze_polyglot_dependencies, scanner::scan_repository,
    FileAstCache,
};
use forge_core::{
    config::load_config, scoring::calculate_health_score, Finding, ForgeError, Repository,
};
use forge_rules::RuleEngine;
use forge_security::SecurityScanner;
use serde::{Deserialize, Serialize};

use crate::cli::OutputFormat;
use crate::theme::Theme;

/// Single evaluation tick emitted by `eforge watch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchTickReport {
    pub tick_index: usize,
    pub elapsed_ms: u64,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub recomputed_files: Vec<String>,
    pub health_score: u8,
    pub health_delta: i16,
    pub cycle_count: usize,
    pub finding_count: usize,
    pub top_findings: Vec<Finding>,
}

/// Executes a single incremental radar evaluation pass against `repo_root` using `ast_cache`.
pub fn execute_watch_tick(
    repo_root: &Path,
    ast_cache: &mut FileAstCache,
    tick_index: usize,
    previous_health: Option<u8>,
) -> Result<WatchTickReport, ForgeError> {
    let start = Instant::now();
    let config = load_config(repo_root)?;
    let discovery = scan_repository(repo_root, &config.ignore_patterns)?;
    let inc = analyze_architecture_incremental(repo_root, &discovery.files, ast_cache)?;
    let dep_report =
        analyze_polyglot_dependencies(repo_root, &discovery.files, &inc.graph.external_imports)?;

    let cycle_count = inc.graph.circular_dependencies.len();

    let mut repo = Repository {
        name: config.project_name.clone().unwrap_or_else(|| {
            repo_root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("repository")
                .to_string()
        }),
        root_path: repo_root.to_string_lossy().replace('\\', "/"),
        detected_languages: discovery.languages,
        detected_frameworks: discovery.frameworks,
        package_managers: discovery.package_managers,
        modules: inc.graph.modules,
        files: discovery.files,
        dependencies: dep_report.dependencies,
        findings: Vec::new(),
        health: None,
    };

    let engine = RuleEngine::from_config(repo_root, &config)?;
    let mut findings = engine.evaluate(&repo)?;
    findings.extend(dep_report.findings);
    findings.extend(SecurityScanner::scan_repository_files_with_config(
        repo_root,
        &repo.files,
        config.security.as_ref(),
    )?);
    findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
    repo.findings = findings;

    let health = calculate_health_score(&repo);
    let health_score = health.overall;
    let health_delta = match previous_health {
        Some(prev) => health_score as i16 - prev as i16,
        None => 0,
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let finding_count = repo.findings.len();
    let top_findings = repo.findings.into_iter().take(5).collect();

    Ok(WatchTickReport {
        tick_index,
        elapsed_ms,
        cache_hits: inc.cache_hits,
        cache_misses: inc.cache_misses,
        recomputed_files: inc.recomputed_files,
        health_score,
        health_delta,
        cycle_count,
        finding_count,
        top_findings,
    })
}

/// Renders a [`WatchTickReport`] in the requested [`OutputFormat`].
pub fn render_watch_tick(
    report: &WatchTickReport,
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
                "### ELCODE Forge Watch Radar (Tick #{})\n\n",
                report.tick_index
            ));
            md.push_str(&format!(
                "- **Health Score:** {}/100 (delta: {:+})\n- **Latency:** {} ms (BLAKE3 AST Cache: {} hit / {} miss)\n- **Cycles:** {}\n- **Findings:** {}\n",
                report.health_score,
                report.health_delta,
                report.elapsed_ms,
                report.cache_hits,
                report.cache_misses,
                report.cycle_count,
                report.finding_count
            ));
            Ok(md)
        }
        OutputFormat::Terminal => {
            let theme = Theme::default();
            let mut out = String::new();
            let delta_str = match report.health_delta.cmp(&0) {
                std::cmp::Ordering::Greater => theme.green(&format!("(+{})", report.health_delta)),
                std::cmp::Ordering::Less => theme.red(&format!("({})", report.health_delta)),
                std::cmp::Ordering::Equal => theme.dim("(±0)"),
            };
            out.push_str(&format!(
                "⚡ {} [Tick #{}] ── {} ms (AST cache: {} hit, {} miss)\n",
                theme.bold("ELCODE Forge Live Radar"),
                report.tick_index,
                report.elapsed_ms,
                report.cache_hits,
                report.cache_misses
            ));
            out.push_str(&format!(
                "   Health: {} {} │ SCC Cycles: {} │ Active Findings: {}\n",
                theme.render_score_meter(report.health_score),
                delta_str,
                report.cycle_count,
                report.finding_count
            ));
            if !report.recomputed_files.is_empty() {
                out.push_str(&format!(
                    "   Recomputed ({}): {}\n",
                    report.recomputed_files.len(),
                    report
                        .recomputed_files
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            for f in &report.top_findings {
                let loc = f
                    .location
                    .as_ref()
                    .map(|l| format!("{}:{}", l.file_path, l.start_line.unwrap_or(1)))
                    .unwrap_or_else(|| "-".to_string());
                out.push_str(&format!(
                    "   • {} {} at {} — {}\n",
                    theme.badge_severity(f.severity),
                    theme.bold(&f.rule_id),
                    theme.cyan(&loc),
                    f.title
                ));
            }
            Ok(out)
        }
    }
}

/// Runs `eforge watch`. When `once` is `true`, performs an initial warm-up pass and one
/// incremental cache-verified pass and returns immediately.
pub fn run_watch(
    repo_root: &Path,
    debounce_ms: u64,
    once: bool,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    let mut ast_cache = FileAstCache::load(repo_root);
    let first = execute_watch_tick(repo_root, &mut ast_cache, 1, None)?;
    let _ = ast_cache.save(repo_root);

    if once {
        // Execute second warm pass to verify incremental cache hit behavior
        let warm = execute_watch_tick(repo_root, &mut ast_cache, 2, Some(first.health_score))?;
        return render_watch_tick(&warm, format);
    }

    println!("{}", render_watch_tick(&first, format)?);
    let mut prev_health = first.health_score;
    let mut tick_idx = 1usize;
    let poll_interval = Duration::from_millis(debounce_ms.max(25));

    loop {
        thread::sleep(poll_interval);
        let config = load_config(repo_root)?;
        let discovery = scan_repository(repo_root, &config.ignore_patterns)?;
        let has_changes = discovery.files.iter().any(|f| {
            ast_cache
                .entries
                .get(&f.relative_path)
                .is_some_and(|cached| cached.content_hash != f.content_hash)
        });
        if has_changes {
            tick_idx += 1;
            let tick = execute_watch_tick(repo_root, &mut ast_cache, tick_idx, Some(prev_health))?;
            prev_health = tick.health_score;
            let _ = ast_cache.save(repo_root);
            println!("{}", render_watch_tick(&tick, format)?);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn watch_once_executes_warm_incremental_pass_with_full_cache_hits() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let mut cache = FileAstCache::default();
        let cold = execute_watch_tick(&fixture, &mut cache, 1, None);
        assert!(cold.is_ok());
        let Ok(t1) = cold else { return };
        assert!(t1.cache_misses > 0);

        let warm = execute_watch_tick(&fixture, &mut cache, 2, Some(t1.health_score));
        assert!(warm.is_ok());
        let Ok(t2) = warm else { return };
        assert_eq!(t2.cache_misses, 0);
        assert_eq!(t2.cache_hits, t1.cache_misses);
        assert_eq!(t2.health_delta, 0);
    }
}
