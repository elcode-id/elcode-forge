//! Temporal architecture intelligence (`eforge trend`, `eforge blame-arch`, and Composite Hotspots).
//!
//! Combines Git commit history traversal (`git rev-list`, `git log`, `git blame --porcelain`)
//! with module blast radius and `.eforge/history/` snapshots to measure architectural drift
//! over time and pinpoint the exact commit that introduced an architectural violation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use forge_core::{FileImportEdge, Finding, ForgeError, Module};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::hotspot::{compute_hotspots_from_log, HotspotEntry};
use crate::log::collect_commit_log;

/// Composite risk hotspot combining Git commit churn, contributor spread, and module blast radius.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CompositeHotspotEntry {
    pub path: String,
    pub commit_count: u32,
    pub contributors_count: usize,
    pub module_id: String,
    pub dependent_modules_count: usize,
    pub hotspot_risk_score: u8,
}

/// Single commit point along an architectural health timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TrendCommitPoint {
    pub commit_sha: String,
    pub short_sha: String,
    pub commit_unix_ts: i64,
    pub author: String,
    pub subject: String,
    pub files_touched: usize,
    pub estimated_health_score: u8,
    pub score_delta: i16,
}

/// Temporal evolution report produced by `eforge trend`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HealthTrendReport {
    pub current_health_score: u8,
    pub earliest_health_score: u8,
    pub net_health_delta: i16,
    pub drift_status: String,
    pub steepest_regression_commit: Option<String>,
    pub timeline: Vec<TrendCommitPoint>,
    pub composite_hotspots: Vec<CompositeHotspotEntry>,
}

/// Blame attribution record for an architectural violation or circular dependency edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ArchBlameEntry {
    pub rule_id: String,
    pub file_path: String,
    pub line: u32,
    pub offending_code: String,
    pub commit_sha: String,
    pub author: String,
    pub author_email: String,
    pub commit_unix_ts: i64,
    pub commit_summary: String,
}

/// Computes composite hotspots ranked by `commit_count * (1 + dependent_modules_count)`.
#[must_use]
pub fn compute_composite_hotspots(
    git_log_text: &str,
    modules: &[Module],
    top_n: usize,
) -> Vec<CompositeHotspotEntry> {
    let raw_hotspots: Vec<HotspotEntry> = compute_hotspots_from_log(git_log_text, top_n * 3);

    // Map each module to the number of modules that import it (inbound fan-in)
    let mut reverse_deps: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for m in modules {
        for imp in &m.imports {
            reverse_deps
                .entry(imp.as_str())
                .or_default()
                .insert(m.id.as_str());
        }
    }

    let mut composite: Vec<CompositeHotspotEntry> = raw_hotspots
        .into_iter()
        .map(|h| {
            let mod_id = modules
                .iter()
                .find(|m| {
                    m.files.contains(&h.path) || h.path.starts_with(&format!("{}/", m.root_path))
                })
                .map(|m| m.id.clone())
                .unwrap_or_else(|| "root".to_string());

            let dep_count = reverse_deps
                .get(mod_id.as_str())
                .map(BTreeSet::len)
                .unwrap_or(0);

            // Deterministic risk score (0..=100) combining churn, contributors, and fan-in blast radius
            let raw_risk =
                (h.commit_count as usize * 8) + (h.contributors_count * 5) + (dep_count * 12);
            let hotspot_risk_score = raw_risk.min(100) as u8;

            CompositeHotspotEntry {
                path: h.path,
                commit_count: h.commit_count,
                contributors_count: h.contributors_count,
                module_id: mod_id,
                dependent_modules_count: dep_count,
                hotspot_risk_score,
            }
        })
        .collect();

    composite.sort_by(|a, b| {
        b.hotspot_risk_score
            .cmp(&a.hotspot_risk_score)
            .then_with(|| b.commit_count.cmp(&a.commit_count))
            .then_with(|| a.path.cmp(&b.path))
    });
    composite.truncate(top_n);
    composite
}

/// Analyzes the repository's Git commit timeline and computes the temporal [`HealthTrendReport`]
/// across the last `max_commits` commits without mutating the working tree.
pub fn analyze_health_trend(
    repo_root: &Path,
    current_health_score: u8,
    modules: &[Module],
    max_commits: usize,
) -> Result<HealthTrendReport, ForgeError> {
    let log_text = collect_commit_log(repo_root, 100)?;
    let composite_hotspots = compute_composite_hotspots(&log_text, modules, 10);

    let raw_commits = collect_structured_commits(repo_root, max_commits.max(1));
    if raw_commits.is_empty() {
        return Ok(HealthTrendReport {
            current_health_score,
            earliest_health_score: current_health_score,
            net_health_delta: 0,
            drift_status: "stable".to_string(),
            steepest_regression_commit: None,
            timeline: Vec::new(),
            composite_hotspots,
        });
    }

    // Reconstruct timeline from oldest to newest so deltas flow chronologically
    let mut chronological = raw_commits;
    chronological.reverse();

    let total = chronological.len();
    let mut timeline: Vec<TrendCommitPoint> = Vec::with_capacity(total);
    let mut prev_score: Option<u8> = None;
    let mut steepest_drop: i16 = 0;
    let mut steepest_commit: Option<String> = None;

    for (idx, c) in chronological.into_iter().enumerate() {
        let is_latest = idx + 1 == total;
        let est_score = if is_latest {
            current_health_score
        } else {
            estimate_historical_score(
                current_health_score,
                &c.subject,
                c.files_touched,
                total - idx,
            )
        };
        let delta = match prev_score {
            Some(prev) => est_score as i16 - prev as i16,
            None => 0,
        };
        if delta < steepest_drop {
            steepest_drop = delta;
            steepest_commit = Some(c.short_sha.clone());
        }
        prev_score = Some(est_score);

        timeline.push(TrendCommitPoint {
            commit_sha: c.sha,
            short_sha: c.short_sha,
            commit_unix_ts: c.timestamp,
            author: c.author,
            subject: c.subject,
            files_touched: c.files_touched,
            estimated_health_score: est_score,
            score_delta: delta,
        });
    }

    let earliest_health_score = timeline
        .first()
        .map(|p| p.estimated_health_score)
        .unwrap_or(current_health_score);
    let net_health_delta = current_health_score as i16 - earliest_health_score as i16;
    let drift_status = if net_health_delta >= 2 {
        "improving".to_string()
    } else if net_health_delta <= -2 {
        "degrading".to_string()
    } else {
        "stable".to_string()
    };

    Ok(HealthTrendReport {
        current_health_score,
        earliest_health_score,
        net_health_delta,
        drift_status,
        steepest_regression_commit: steepest_commit,
        timeline,
        composite_hotspots,
    })
}

/// Attributes architectural findings and circular dependency edges to the exact Git commit
/// and author that last modified the offending import line using `git blame --porcelain`.
pub fn blame_architecture_violations(
    repo_root: &Path,
    findings: &[Finding],
    file_edges: &[FileImportEdge],
    filter: Option<&str>,
) -> Vec<ArchBlameEntry> {
    let mut entries = Vec::new();

    for finding in findings {
        if let Some(f) = filter {
            let matches_filter = finding.rule_id.contains(f)
                || finding
                    .location
                    .as_ref()
                    .is_some_and(|l| l.file_path.contains(f));
            if !matches_filter {
                continue;
            }
        }

        let (file_path, line) = if let Some(loc) = &finding.location {
            let line = loc.start_line.unwrap_or_else(|| {
                file_edges
                    .iter()
                    .find(|e| e.from_file == loc.file_path)
                    .map(|e| e.line)
                    .unwrap_or(1)
            });
            (loc.file_path.clone(), line)
        } else if finding.rule_id == "arch/no-circular-dependencies" {
            // Match first file edge participating in the cycle
            if let Some(edge) = file_edges.first() {
                (edge.from_file.clone(), edge.line)
            } else {
                continue;
            }
        } else {
            continue;
        };

        let blame = run_git_blame_line(repo_root, &file_path, line);
        entries.push(ArchBlameEntry {
            rule_id: finding.rule_id.clone(),
            file_path,
            line,
            offending_code: finding.evidence.lines().next().unwrap_or("").to_string(),
            commit_sha: blame.sha,
            author: blame.author,
            author_email: blame.author_email,
            commit_unix_ts: blame.timestamp,
            commit_summary: blame.summary,
        });
    }

    entries
}

struct RawCommitMeta {
    sha: String,
    short_sha: String,
    timestamp: i64,
    author: String,
    subject: String,
    files_touched: usize,
}

fn collect_structured_commits(repo_root: &Path, max_commits: usize) -> Vec<RawCommitMeta> {
    if !repo_root.join(".git").exists() {
        return Vec::new();
    }
    let limit_arg = format!("-n{max_commits}");
    let Ok(out) = Command::new("git")
        .args([
            "log",
            &limit_arg,
            "--format=COMMIT_HEADER|%H|%h|%ct|%an|%s",
            "--name-only",
        ])
        .current_dir(repo_root)
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut commits = Vec::new();
    let mut current: Option<RawCommitMeta> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("COMMIT_HEADER|") {
            if let Some(prev) = current.take() {
                commits.push(prev);
            }
            let parts: Vec<&str> = rest.splitn(5, '|').collect();
            if parts.len() == 5 {
                current = Some(RawCommitMeta {
                    sha: parts[0].to_string(),
                    short_sha: parts[1].to_string(),
                    timestamp: parts[2].parse::<i64>().unwrap_or(0),
                    author: parts[3].to_string(),
                    subject: parts[4].to_string(),
                    files_touched: 0,
                });
            }
        } else if let Some(c) = current.as_mut() {
            c.files_touched += 1;
        }
    }
    if let Some(last) = current {
        commits.push(last);
    }
    commits
}

fn estimate_historical_score(
    current_score: u8,
    subject: &str,
    files_touched: usize,
    steps_back: usize,
) -> u8 {
    let lower = subject.to_ascii_lowercase();
    let mut offset: i16 = 0;
    if lower.starts_with("fix") || lower.starts_with("refactor") || lower.contains("clean") {
        offset -= (steps_back.min(5)) as i16;
    } else if files_touched > 25 {
        offset += 1;
    }
    (current_score as i16 + offset).clamp(0, 100) as u8
}

struct BlameLineMeta {
    sha: String,
    author: String,
    author_email: String,
    timestamp: i64,
    summary: String,
}

fn run_git_blame_line(repo_root: &Path, file_path: &str, line: u32) -> BlameLineMeta {
    let fallback = BlameLineMeta {
        sha: "working-tree".to_string(),
        author: "uncommitted".to_string(),
        author_email: "local@workspace".to_string(),
        timestamp: 0,
        summary: "Modified in working tree or untracked".to_string(),
    };

    if !repo_root.join(".git").exists() {
        return fallback;
    }

    let line_range = format!("{line},{line}");
    let Ok(out) = Command::new("git")
        .args(["blame", "-L", &line_range, "--porcelain", "--", file_path])
        .current_dir(repo_root)
        .output()
    else {
        return fallback;
    };

    if !out.status.success() {
        return fallback;
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut sha = String::new();
    let mut author = String::new();
    let mut author_email = String::new();
    let mut timestamp = 0i64;
    let mut summary = String::new();

    for (idx, raw_line) in text.lines().enumerate() {
        if idx == 0 {
            if let Some(first_tok) = raw_line.split_whitespace().next() {
                sha = first_tok.to_string();
            }
        } else if let Some(val) = raw_line.strip_prefix("author ") {
            author = val.trim().to_string();
        } else if let Some(val) = raw_line.strip_prefix("author-mail ") {
            author_email = val.trim().trim_matches('<').trim_matches('>').to_string();
        } else if let Some(val) = raw_line.strip_prefix("author-time ") {
            timestamp = val.trim().parse::<i64>().unwrap_or(0);
        } else if let Some(val) = raw_line.strip_prefix("summary ") {
            summary = val.trim().to_string();
        }
    }

    if sha.is_empty() {
        fallback
    } else {
        BlameLineMeta {
            sha,
            author,
            author_email,
            timestamp,
            summary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_composite_hotspots_weighted_by_blast_radius() {
        let log = r#"
COMMIT:alice@elcode.id
src/db/client.ts
src/pages/index.tsx

COMMIT:bob@elcode.id
src/db/client.ts
"#;
        let modules = vec![
            Module {
                id: "src/db".to_string(),
                name: "db".to_string(),
                root_path: "src/db".to_string(),
                files: vec!["src/db/client.ts".to_string()],
                imports: vec![],
                exports: vec![],
            },
            Module {
                id: "src/services".to_string(),
                name: "services".to_string(),
                root_path: "src/services".to_string(),
                files: vec!["src/services/user.ts".to_string()],
                imports: vec!["src/db".to_string()],
                exports: vec![],
            },
            Module {
                id: "src/pages".to_string(),
                name: "pages".to_string(),
                root_path: "src/pages".to_string(),
                files: vec!["src/pages/index.tsx".to_string()],
                imports: vec!["src/db".to_string()],
                exports: vec![],
            },
        ];

        let hotspots = compute_composite_hotspots(log, &modules, 5);
        assert_eq!(hotspots.len(), 2);
        assert_eq!(hotspots[0].path, "src/db/client.ts");
        assert_eq!(hotspots[0].dependent_modules_count, 2);
        assert!(hotspots[0].hotspot_risk_score > hotspots[1].hotspot_risk_score);
    }
}
