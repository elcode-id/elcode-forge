use std::path::Path;
use std::process::Command;

use forge_core::ForgeError;
use serde::{Deserialize, Serialize};

/// Normalized file-level change extracted from a unified Git diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub added_lines: u32,
    pub deleted_lines: u32,
    pub is_new: bool,
    pub is_deleted: bool,
}

/// Parses a standard unified Git diff (`git diff`) into deterministically sorted [`FileChange`] items.
pub fn parse_unified_diff(diff_text: &str) -> Result<Vec<FileChange>, ForgeError> {
    let mut changes: Vec<FileChange> = Vec::new();
    let mut current: Option<FileChange> = None;

    for raw_line in diff_text.lines() {
        if let Some(rest) = raw_line.strip_prefix("diff --git ") {
            if let Some(finished) = current.take() {
                if !finished.path.is_empty() {
                    changes.push(finished);
                }
            }
            let path = parse_diff_header_path(rest).unwrap_or_default();
            current = Some(FileChange {
                path,
                added_lines: 0,
                deleted_lines: 0,
                is_new: false,
                is_deleted: false,
            });
            continue;
        }

        if raw_line.starts_with("new file mode ") {
            if let Some(c) = current.as_mut() {
                c.is_new = true;
            }
            continue;
        }

        if raw_line.starts_with("deleted file mode ") {
            if let Some(c) = current.as_mut() {
                c.is_deleted = true;
            }
            continue;
        }

        if let Some(rest) = raw_line.strip_prefix("--- ") {
            if rest.trim() == "/dev/null" {
                if let Some(c) = current.as_mut() {
                    c.is_new = true;
                }
            }
            continue;
        }

        if let Some(rest) = raw_line.strip_prefix("+++ ") {
            let trimmed = rest.trim();
            if trimmed == "/dev/null" {
                if let Some(c) = current.as_mut() {
                    c.is_deleted = true;
                }
            } else {
                let cleaned = trimmed.strip_prefix("b/").unwrap_or(trimmed).to_string();
                let c = current.get_or_insert(FileChange {
                    path: cleaned.clone(),
                    added_lines: 0,
                    deleted_lines: 0,
                    is_new: false,
                    is_deleted: false,
                });
                c.path = cleaned;
            }
            continue;
        }

        if raw_line.starts_with("@@") {
            continue;
        }

        if raw_line.starts_with('+') && !raw_line.starts_with("+++") {
            if let Some(c) = current.as_mut() {
                c.added_lines += 1;
            }
        } else if raw_line.starts_with('-') && !raw_line.starts_with("---") {
            if let Some(c) = current.as_mut() {
                c.deleted_lines += 1;
            }
        }
    }

    if let Some(finished) = current.take() {
        if !finished.path.is_empty() {
            changes.push(finished);
        }
    }

    changes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(changes)
}

/// Runs a read-only `git diff HEAD` inside `repo_root` to capture uncommitted working-tree changes.
///
/// If `repo_root` is not a Git repository or `HEAD` does not exist yet, returns `Ok(String::new())`.
pub fn collect_working_tree_diff(repo_root: &Path) -> Result<String, ForgeError> {
    if !repo_root.join(".git").exists() {
        return Ok(String::new());
    }

    let output = Command::new("git")
        .args(["diff", "HEAD", "--unified=0"])
        .current_dir(repo_root)
        .output();

    match output {
        Ok(out) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        _ => Ok(String::new()),
    }
}

fn parse_diff_header_path(rest: &str) -> Option<String> {
    let (_, b_part) = rest.trim().rsplit_once(' ')?;
    let cleaned = b_part.strip_prefix("b/").unwrap_or(b_part);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multi_file_unified_diff_deterministically() {
        let sample_diff = r#"diff --git a/src/services/userService.ts b/src/services/userService.ts
index 1111111..2222222 100644
--- a/src/services/userService.ts
+++ b/src/services/userService.ts
@@ -1,4 +1,5 @@
 import { queryDb } from '../db/client';
-export function oldFn() {}
+export function newFn() {
+  return true;
+}
diff --git a/src/components/NewBadge.tsx b/src/components/NewBadge.tsx
new file mode 100644
--- /dev/null
+++ b/src/components/NewBadge.tsx
@@ -0,0 +1,2 @@
+export const NewBadge = () => null;
+export default NewBadge;
"#;

        let parsed = parse_unified_diff(sample_diff);
        assert!(parsed.is_ok());

        if let Ok(changes) = parsed {
            assert_eq!(changes.len(), 2);
            assert_eq!(
                changes[0],
                FileChange {
                    path: "src/components/NewBadge.tsx".to_string(),
                    added_lines: 2,
                    deleted_lines: 0,
                    is_new: true,
                    is_deleted: false,
                }
            );
            assert_eq!(
                changes[1],
                FileChange {
                    path: "src/services/userService.ts".to_string(),
                    added_lines: 3,
                    deleted_lines: 1,
                    is_new: false,
                    is_deleted: false,
                }
            );
        }
    }
}
