//! Per-commit deterministic history snapshot collection and trends retrieval.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use forge_core::cache::EFORGE_DIR;
use forge_core::ForgeError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::commands::report::ReportBundle;

/// Subdirectory under `.eforge/` where history snapshots are stored.
pub const HISTORY_DIR: &str = "history";

/// Deterministic health and findings snapshot for a single commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HistorySnapshot {
    pub head_sha: String,
    pub commit_unix_ts: i64,
    pub overall: u8,
    pub dimensions: BTreeMap<String, u8>,
    pub findings_by_severity: BTreeMap<String, usize>,
}

/// Saves a deterministic [`HistorySnapshot`] into `.eforge/history/<commit_unix_ts>-<sha12>.json`.
///
/// Returns `Ok(Some(PathBuf))` on write, or `Ok(None)` if `bundle.revision` is absent.
/// Operation is idempotent per commit revision.
pub fn save_history_snapshot(
    repo_root: &Path,
    bundle: &ReportBundle,
) -> Result<Option<PathBuf>, ForgeError> {
    let rev = match &bundle.revision {
        Some(r) => r,
        None => return Ok(None),
    };

    let history_dir = repo_root.join(EFORGE_DIR).join(HISTORY_DIR);
    fs::create_dir_all(&history_dir).map_err(|source| ForgeError::Io {
        path: history_dir.clone(),
        source,
    })?;

    let short_sha = if rev.head_sha.len() >= 12 {
        &rev.head_sha[..12]
    } else {
        &rev.head_sha
    };
    let filename = format!("{}-{}.json", rev.commit_unix_ts, short_sha);
    let target_file = history_dir.join(filename);

    let mut dimensions = BTreeMap::new();
    let overall = if let Some(health) = &bundle.repository.health {
        for dim in &health.dimensions {
            dimensions.insert(dim.dimension.clone(), dim.score);
        }
        health.overall
    } else {
        100
    };

    let mut findings_by_severity = BTreeMap::new();
    for f in &bundle.repository.findings {
        let sev_str = format!("{:?}", f.severity).to_lowercase();
        *findings_by_severity.entry(sev_str).or_insert(0) += 1;
    }

    let snapshot = HistorySnapshot {
        head_sha: rev.head_sha.clone(),
        commit_unix_ts: rev.commit_unix_ts,
        overall,
        dimensions,
        findings_by_severity,
    };

    let rendered =
        serde_json::to_string_pretty(&snapshot).map_err(|err| ForgeError::Serialization {
            message: format!("Failed to serialize HistorySnapshot: {err}"),
        })?;

    fs::write(&target_file, rendered).map_err(|source| ForgeError::Io {
        path: target_file.clone(),
        source,
    })?;

    Ok(Some(target_file))
}

/// Loads history snapshots from `.eforge/history/`, ordered by `commit_unix_ts` ascending.
///
/// Malformed files are silently skipped to preserve resilience.
/// If `limit > 0`, returns at most the last `limit` snapshots.
pub fn load_history(repo_root: &Path, limit: usize) -> Result<Vec<HistorySnapshot>, ForgeError> {
    let history_dir = repo_root.join(EFORGE_DIR).join(HISTORY_DIR);
    if !history_dir.is_dir() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&history_dir).map_err(|source| ForgeError::Io {
        path: history_dir.clone(),
        source,
    })?;

    let mut snapshots = Vec::new();

    for entry_res in entries {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(snapshot) = serde_json::from_str::<HistorySnapshot>(&content) {
                snapshots.push(snapshot);
            }
        }
    }

    // Sort deterministically by commit_unix_ts ascending, then head_sha ascending
    snapshots.sort_by(|a, b| {
        a.commit_unix_ts
            .cmp(&b.commit_unix_ts)
            .then_with(|| a.head_sha.cmp(&b.head_sha))
    });

    if limit > 0 && snapshots.len() > limit {
        let start = snapshots.len() - limit;
        snapshots = snapshots.split_off(start);
    }

    Ok(snapshots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::report::{build_report_bundle, ReportOptions};
    use forge_git::GitRevision;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = env::temp_dir().join(format!("eforge_history_{label}_{nanos}"));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn test_save_and_load_history_snapshot_cycle() -> Result<(), ForgeError> {
        let root = unique_temp_dir("cycle");

        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let opts = ReportOptions {
            hotspots_top_n: 5,
            include_git: false,
        };
        let mut bundle = build_report_bundle(&fixture, &opts)?;

        // Without git revision, save returns None
        assert!(bundle.revision.is_none());
        let res = save_history_snapshot(&root, &bundle)?;
        assert!(res.is_none());

        // With mock git revisions
        bundle.revision = Some(GitRevision {
            head_sha: "1111111111111111111111111111111111111111".to_string(),
            commit_unix_ts: 1000,
        });
        let saved1 = save_history_snapshot(&root, &bundle)?;
        assert!(saved1.is_some());

        bundle.revision = Some(GitRevision {
            head_sha: "2222222222222222222222222222222222222222".to_string(),
            commit_unix_ts: 2000,
        });
        let saved2 = save_history_snapshot(&root, &bundle)?;
        assert!(saved2.is_some());

        let history = load_history(&root, 0)?;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].commit_unix_ts, 1000);
        assert_eq!(history[1].commit_unix_ts, 2000);

        // Test limit
        let limited = load_history(&root, 1)?;
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].commit_unix_ts, 2000);

        let _ = fs::remove_dir_all(&root);
        Ok(())
    }
}
