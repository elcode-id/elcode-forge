use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::ForgeError;

/// Resolves `candidate` against `repo_root` and enforces that the resulting path remains strictly
/// inside the canonical `repo_root` boundary.
///
/// Rejects any `..` traversal or symlink escape outside `repo_root` with
/// [`ForgeError::PathTraversal`].
pub fn resolve_safe_path(repo_root: &Path, candidate: &Path) -> Result<PathBuf, ForgeError> {
    let canonical_root = fs::canonicalize(repo_root).map_err(|source| ForgeError::Io {
        path: repo_root.to_path_buf(),
        source,
    })?;

    let combined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        canonical_root.join(candidate)
    };

    // Lexical normalization that tracks whether components ever escape `canonical_root`.
    let mut normalized = PathBuf::new();
    for component in combined.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if !normalized.pop() || !normalized.starts_with(&canonical_root) {
                    return Err(ForgeError::PathTraversal {
                        repo_root: repo_root.to_path_buf(),
                        candidate: candidate.to_path_buf(),
                    });
                }
            }
        }
    }

    if !normalized.starts_with(&canonical_root) {
        return Err(ForgeError::PathTraversal {
            repo_root: repo_root.to_path_buf(),
            candidate: candidate.to_path_buf(),
        });
    }

    // If the target path (or an existing ancestor) exists on disk, canonicalize it to prevent
    // symlink escapes outside `canonical_root`.
    if normalized.exists() {
        let canonical_target = fs::canonicalize(&normalized).map_err(|source| ForgeError::Io {
            path: normalized,
            source,
        })?;
        if !canonical_target.starts_with(&canonical_root) {
            return Err(ForgeError::PathTraversal {
                repo_root: repo_root.to_path_buf(),
                candidate: candidate.to_path_buf(),
            });
        }
        return Ok(canonical_target);
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = env::temp_dir().join(format!("eforge_path_guard_{label}_{nanos}"));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn allows_valid_relative_paths_within_repository() {
        let root = unique_temp_dir("valid");
        let sub = root.join("src").join("components");
        assert!(fs::create_dir_all(&sub).is_ok());
        let file = sub.join("UserCard.tsx");
        assert!(fs::write(&file, "export const UserCard = () => null;").is_ok());

        let resolved = resolve_safe_path(&root, Path::new("src/components/UserCard.tsx"));
        assert!(resolved.is_ok());

        let non_existent_inside = resolve_safe_path(&root, Path::new("src/new_module.ts"));
        assert!(non_existent_inside.is_ok());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_parent_directory_traversal_outside_repo_root() {
        let root = unique_temp_dir("traversal");
        let res = resolve_safe_path(&root, Path::new("../../etc/passwd"));
        assert!(matches!(res, Err(ForgeError::PathTraversal { .. })));

        let nested_escape = resolve_safe_path(&root, Path::new("src/../../../secret.txt"));
        assert!(matches!(
            nested_escape,
            Err(ForgeError::PathTraversal { .. })
        ));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_absolute_path_outside_repo_root() {
        let root = unique_temp_dir("abs_root");
        let outside = unique_temp_dir("abs_outside");
        let res = resolve_safe_path(&root, &outside);
        assert!(matches!(res, Err(ForgeError::PathTraversal { .. })));

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }
}
