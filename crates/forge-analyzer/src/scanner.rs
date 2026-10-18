use std::fs;
use std::path::Path;

use forge_core::{FileNode, ForgeCache, ForgeError};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};

use crate::detector::{detect_ecosystem, detect_language, is_test_file};

/// Normalized discovery output produced by scanning a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverySummary {
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub package_managers: Vec<String>,
    pub has_git: bool,
    pub has_docker: bool,
    pub has_github_actions: bool,
    pub has_tests: bool,
    pub files: Vec<FileNode>,
}

/// Built-in directory names that are always skipped during repository scanning.
const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git",
    ".eforge",
    "node_modules",
    "target",
    "dist",
    ".next",
    ".nuxt",
    ".svelte-kit",
];

/// Walks `repo_root` respecting `.gitignore` and `extra_ignores`, computes deterministic file
/// metadata and `blake3` hashes, and detects project languages, frameworks, and package managers.
pub fn scan_repository(
    repo_root: &Path,
    extra_ignores: &[String],
) -> Result<DiscoverySummary, ForgeError> {
    if !repo_root.exists() {
        return Err(ForgeError::Io {
            path: repo_root.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "repository root does not exist",
            ),
        });
    }

    let custom_ignores: Vec<String> = extra_ignores
        .iter()
        .map(|p| {
            p.trim()
                .trim_end_matches("/**")
                .trim_end_matches("/*")
                .trim_matches('/')
                .to_string()
        })
        .filter(|p| !p.is_empty())
        .collect();

    let mut walker = WalkBuilder::new(repo_root);
    walker
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true);

    let root_owned = repo_root.to_path_buf();
    walker.filter_entry(move |entry| {
        if entry.depth() == 0 {
            return true;
        }
        let file_name = entry.file_name().to_str().unwrap_or("");
        if DEFAULT_IGNORED_DIRS.contains(&file_name) {
            return false;
        }
        if let Ok(rel) = entry.path().strip_prefix(&root_owned) {
            let rel_norm = normalize_relative_path(rel);
            for pattern in &custom_ignores {
                if rel_norm == *pattern
                    || rel_norm.starts_with(&format!("{pattern}/"))
                    || file_name == pattern
                {
                    return false;
                }
            }
        }
        true
    });

    let mut files = Vec::new();

    for result in walker.build() {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }

        let path = entry.path();
        let Ok(rel_path) = path.strip_prefix(repo_root) else {
            continue;
        };
        let relative_path = normalize_relative_path(rel_path);
        if relative_path.is_empty() {
            continue;
        }

        let bytes = fs::read(path).map_err(|source| ForgeError::Io {
            path: path.to_path_buf(),
            source,
        })?;

        let size_bytes = bytes.len() as u64;
        let line_count = count_lines(&bytes);
        let content_hash = ForgeCache::hash_bytes(&bytes);
        let language = detect_language(&relative_path);
        let is_test = is_test_file(&relative_path)
            || (language.as_deref() == Some("rust")
                && std::str::from_utf8(&bytes).is_ok_and(|s| s.contains("#[cfg(test)]")));

        files.push(FileNode {
            relative_path,
            language,
            size_bytes,
            line_count,
            content_hash,
            is_test,
        });
    }

    // Sort deterministically by normalized relative_path
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let signals = detect_ecosystem(repo_root, &files);

    Ok(DiscoverySummary {
        languages: signals.languages,
        frameworks: signals.frameworks,
        package_managers: signals.package_managers,
        has_git: signals.has_git,
        has_docker: signals.has_docker,
        has_github_actions: signals.has_github_actions,
        has_tests: signals.has_tests,
        files,
    })
}

fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn count_lines(bytes: &[u8]) -> u32 {
    if bytes.is_empty() {
        return 0;
    }
    let newline_count = bytes.iter().filter(|&&b| b == b'\n').count() as u32;
    if bytes.ends_with(b"\n") {
        newline_count
    } else {
        newline_count + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn scans_fixture_next_app_deterministically_with_sorted_paths() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let summary = scan_repository(&fixture, &[]);
        assert!(summary.is_ok());

        if let Ok(s) = summary {
            assert_eq!(s.languages, vec!["typescript"]);
            assert_eq!(s.frameworks, vec!["nextjs", "react"]);
            assert_eq!(s.package_managers, vec!["npm"]);
            assert!(!s.has_tests);

            let rel_paths: Vec<&str> = s.files.iter().map(|f| f.relative_path.as_str()).collect();
            assert_eq!(
                rel_paths,
                vec![
                    "package.json",
                    "src/components/UserCard.tsx",
                    "src/db/client.ts",
                    "src/pages/index.tsx",
                    "src/services/userService.ts",
                    "tsconfig.json",
                ]
            );
        }
    }

    #[test]
    fn respects_extra_ignore_patterns_during_scan() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let summary = scan_repository(&fixture, &["src/db".to_string()]);
        assert!(summary.is_ok());

        if let Ok(s) = summary {
            assert!(!s
                .files
                .iter()
                .any(|f| f.relative_path.starts_with("src/db")));
        }
    }
}
