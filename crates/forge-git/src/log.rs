use std::path::Path;
use std::process::Command;

use forge_core::ForgeError;
use serde::{Deserialize, Serialize};

/// Git HEAD revision metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRevision {
    pub head_sha: String,
    pub commit_unix_ts: i64,
}

/// Reads the current Git HEAD revision (full SHA and commit unix timestamp).
///
/// Returns `Ok(None)` if `repo_root` is not a Git repository or `git` command fails.
pub fn read_head_revision(repo_root: &Path) -> Result<Option<GitRevision>, ForgeError> {
    if !repo_root.join(".git").exists() {
        return Ok(None);
    }

    let output = Command::new("git")
        .args(["log", "-1", "--format=%H %ct"])
        .current_dir(repo_root)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let trimmed = text.trim();
            if let Some((sha, ts_str)) = trimmed.split_once(' ') {
                if let Ok(ts) = ts_str.parse::<i64>() {
                    return Ok(Some(GitRevision {
                        head_sha: sha.to_string(),
                        commit_unix_ts: ts,
                    }));
                }
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// Collects commit log output formatted for [`crate::hotspot::compute_hotspots_from_log`].
///
/// Uses `git log --format="COMMIT:%ae" --name-only -n <max_commits>`.
/// Returns an empty string if `repo_root` is not a Git repository or `git` fails.
pub fn collect_commit_log(repo_root: &Path, max_commits: usize) -> Result<String, ForgeError> {
    if !repo_root.join(".git").exists() || max_commits == 0 {
        return Ok(String::new());
    }

    let limit_arg = format!("-n{max_commits}");
    let output = Command::new("git")
        .args(["log", "--format=COMMIT:%ae", "--name-only", &limit_arg])
        .current_dir(repo_root)
        .output();

    match output {
        Ok(out) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        _ => Ok(String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_git_dir_returns_none_and_empty() {
        let temp_dir = std::env::temp_dir().join("forge_non_git_test_12345");
        let _ = std::fs::create_dir_all(&temp_dir);

        let rev = read_head_revision(&temp_dir);
        assert!(rev.is_ok());
        assert_eq!(rev.ok().flatten(), None);

        let log = collect_commit_log(&temp_dir, 50);
        assert!(log.is_ok());
        assert_eq!(log.unwrap_or_default(), "");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn current_repo_returns_valid_head_revision() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap_or(Path::new("."));

        if root.join(".git").exists() {
            let rev = read_head_revision(root);
            assert!(rev.is_ok());
            if let Ok(Some(r)) = rev {
                assert!(!r.head_sha.is_empty());
                assert!(r.commit_unix_ts > 0);
            }

            let log = collect_commit_log(root, 5);
            assert!(log.is_ok());
            let log_text = log.unwrap_or_default();
            assert!(log_text.contains("COMMIT:"));
        }
    }
}
