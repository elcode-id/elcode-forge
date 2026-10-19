use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use forge_core::Module;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::diff::FileChange;

/// Blast-radius change impact report for a Git diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ChangeImpactReport {
    pub changed_files: Vec<FileChange>,
    pub directly_affected_modules: Vec<String>,
    pub transitively_affected_modules: Vec<String>,
    pub changed_files_without_tests: Vec<String>,
}

/// Computes the direct and transitive module blast radius for `changes` across `modules`,
/// and identifies modified source files that lack corresponding tests in `test_files`.
pub fn compute_change_impact(
    changes: &[FileChange],
    modules: &[Module],
    test_files: &BTreeSet<String>,
) -> ChangeImpactReport {
    let mut direct_modules = BTreeSet::new();

    for change in changes {
        for module in modules {
            if module.files.contains(&change.path)
                || change.path.starts_with(&format!("{}/", module.root_path))
            {
                direct_modules.insert(module.id.clone());
            }
        }
    }

    // Build reverse dependency adjacency list: imported_mod -> set of consumer_mods
    let mut reverse_deps: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for module in modules {
        for imported in &module.imports {
            reverse_deps
                .entry(imported.as_str())
                .or_default()
                .insert(module.id.as_str());
        }
    }

    // BFS to find all transitively affected consumer modules (excluding direct modules)
    let mut visited: BTreeSet<String> = direct_modules.clone();
    let mut queue: VecDeque<String> = direct_modules.iter().cloned().collect();
    let mut transitive_modules = BTreeSet::new();

    while let Some(curr) = queue.pop_front() {
        if let Some(consumers) = reverse_deps.get(curr.as_str()) {
            for &consumer in consumers {
                if !visited.contains(consumer) {
                    visited.insert(consumer.to_string());
                    transitive_modules.insert(consumer.to_string());
                    queue.push_back(consumer.to_string());
                }
            }
        }
    }

    // Identify modified source files that lack associated test files
    let changed_paths: BTreeSet<&str> = changes.iter().map(|c| c.path.as_str()).collect();
    let mut untested = BTreeSet::new();

    for change in changes {
        if change.is_deleted || !is_code_file(&change.path) || is_test_path(&change.path) {
            continue;
        }
        if !has_matching_test(&change.path, test_files, &changed_paths) {
            untested.insert(change.path.clone());
        }
    }

    ChangeImpactReport {
        changed_files: changes.to_vec(),
        directly_affected_modules: direct_modules.into_iter().collect(),
        transitively_affected_modules: transitive_modules.into_iter().collect(),
        changed_files_without_tests: untested.into_iter().collect(),
    }
}

fn is_code_file(path: &str) -> bool {
    matches!(
        Path::new(path).extension().and_then(|e| e.to_str()),
        Some("ts" | "tsx" | "js" | "jsx" | "rs" | "py" | "go" | "php")
    )
}

fn is_test_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.contains("_test.")
        || lower.starts_with("tests/")
        || lower.contains("/tests/")
        || lower.contains("/__tests__/")
}

fn has_matching_test(
    source_path: &str,
    test_files: &BTreeSet<String>,
    changed_paths: &BTreeSet<&str>,
) -> bool {
    let Some(stem) = Path::new(source_path).file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    let stem_lower = stem.to_ascii_lowercase();

    let matches_stem = |candidate: &str| {
        let lower = candidate.to_ascii_lowercase();
        is_test_path(&lower) && lower.contains(&stem_lower)
    };

    test_files.iter().any(|t| matches_stem(t)) || changed_paths.iter().any(|c| matches_stem(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_direct_and_transitive_blast_radius_and_missing_tests() {
        let modules = vec![
            Module {
                id: "src/db".to_string(),
                name: "db".to_string(),
                root_path: "src/db".to_string(),
                files: vec!["src/db/client.ts".to_string()],
                imports: vec![],
                exports: vec!["queryDb".to_string()],
            },
            Module {
                id: "src/services".to_string(),
                name: "services".to_string(),
                root_path: "src/services".to_string(),
                files: vec!["src/services/userService.ts".to_string()],
                imports: vec!["src/db".to_string()],
                exports: vec!["getActiveUsers".to_string()],
            },
            Module {
                id: "src/pages".to_string(),
                name: "pages".to_string(),
                root_path: "src/pages".to_string(),
                files: vec!["src/pages/index.tsx".to_string()],
                imports: vec!["src/services".to_string()],
                exports: vec!["default".to_string()],
            },
        ];

        let changes = vec![FileChange {
            path: "src/db/client.ts".to_string(),
            added_lines: 4,
            deleted_lines: 1,
            is_new: false,
            is_deleted: false,
        }];

        let test_files = BTreeSet::new();
        let report = compute_change_impact(&changes, &modules, &test_files);

        assert_eq!(report.directly_affected_modules, vec!["src/db"]);
        assert_eq!(
            report.transitively_affected_modules,
            vec!["src/pages", "src/services"]
        );
        assert_eq!(report.changed_files_without_tests, vec!["src/db/client.ts"]);
    }
}
