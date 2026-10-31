//! Smart Impacted Test Selection engine (`eforge review --impacted-tests` & `eforge_get_impacted_tests`).
//!
//! Maps a set of Git [`FileChange`]s through both file-level [`FileImportEdge`] reverse BFS
//! and module-level transitive blast radius to identify Tier 1 (directly modified),
//! Tier 2 (direct companion & file-graph importer), and Tier 3 (transitive blast-radius)
//! test files, and synthesizes ready-to-run test runner commands.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use forge_core::{FileImportEdge, Module};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::diff::FileChange;
use crate::impact::compute_change_impact;

/// Tiered impacted test selection report for a Git diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ImpactedTestsReport {
    pub tier1_modified_tests: Vec<String>,
    pub tier2_direct_tests: Vec<String>,
    pub tier3_transitive_tests: Vec<String>,
    pub all_selected_tests: Vec<String>,
    pub uncovered_changed_files: Vec<String>,
    pub recommended_commands: Vec<String>,
}

/// Computes the Tier 1, Tier 2, and Tier 3 impacted test files for `changes` and synthesizes
/// ready-to-run test runner commands for the repository's detected ecosystems.
#[must_use]
#[allow(clippy::collapsible_if)]
pub fn compute_impacted_tests(
    changes: &[FileChange],
    modules: &[Module],
    file_edges: &[FileImportEdge],
    test_files: &BTreeSet<String>,
    detected_languages: &[String],
    package_managers: &[String],
) -> ImpactedTestsReport {
    let impact = compute_change_impact(changes, modules, test_files);

    // 1. Tier 1: Test files directly modified in the diff
    let mut tier1 = BTreeSet::new();
    let mut changed_source_files = BTreeSet::new();

    for change in changes {
        if change.is_deleted {
            continue;
        }
        if test_files.contains(&change.path) || is_test_like(&change.path) {
            tier1.insert(change.path.clone());
        } else {
            changed_source_files.insert(change.path.clone());
        }
    }

    // Build reverse file import graph: target_file -> set of importer_files
    let mut reverse_file_edges: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for edge in file_edges {
        reverse_file_edges
            .entry(edge.to_file.as_str())
            .or_default()
            .insert(edge.from_file.as_str());
    }

    // 2. Tier 2: Companion tests + direct/multi-hop file-graph test importers
    let mut tier2 = BTreeSet::new();
    for src in &changed_source_files {
        // Companion stem matching
        if let Some(stem) = Path::new(src).file_stem().and_then(|s| s.to_str()) {
            let stem_lower = stem.to_ascii_lowercase();
            for candidate_test in test_files {
                if !tier1.contains(candidate_test)
                    && candidate_test.to_ascii_lowercase().contains(&stem_lower)
                {
                    tier2.insert(candidate_test.clone());
                }
            }
        }
    }

    // File-level BFS from changed source files to find any importing test files
    let mut visited_files: BTreeSet<&str> =
        changed_source_files.iter().map(String::as_str).collect();
    let mut file_queue: VecDeque<(&str, usize)> = changed_source_files
        .iter()
        .map(|s| (s.as_str(), 0))
        .collect();

    while let Some((curr_file, depth)) = file_queue.pop_front() {
        if let Some(importers) = reverse_file_edges.get(curr_file) {
            for &importer in importers {
                if test_files.contains(importer) || is_test_like(importer) {
                    if !tier1.contains(importer) {
                        if depth == 0 {
                            tier2.insert(importer.to_string());
                        }
                    }
                }
                if !visited_files.contains(importer) && depth < 4 {
                    visited_files.insert(importer);
                    file_queue.push_back((importer, depth + 1));
                }
            }
        }
    }

    // 3. Tier 3: Tests residing inside or importing from directly/transitively affected modules
    let mut tier3 = BTreeSet::new();
    let affected_mods: BTreeSet<&str> = impact
        .directly_affected_modules
        .iter()
        .chain(impact.transitively_affected_modules.iter())
        .map(String::as_str)
        .collect();

    for test_file in test_files {
        if tier1.contains(test_file) || tier2.contains(test_file) {
            continue;
        }
        let in_affected_mod = affected_mods
            .iter()
            .any(|m| test_file.starts_with(&format!("{m}/")) || *m == "root");
        if in_affected_mod {
            tier3.insert(test_file.clone());
        }
    }

    // Also include file-graph importers found at depth >= 1 in Tier 3
    for visited in visited_files {
        if (test_files.contains(visited) || is_test_like(visited))
            && !tier1.contains(visited)
            && !tier2.contains(visited)
        {
            tier3.insert(visited.to_string());
        }
    }

    let mut all_selected = BTreeSet::new();
    all_selected.extend(tier1.iter().cloned());
    all_selected.extend(tier2.iter().cloned());
    all_selected.extend(tier3.iter().cloned());

    let all_selected_vec: Vec<String> = all_selected.into_iter().collect();
    let recommended_commands = synthesize_test_commands(
        &all_selected_vec,
        &impact.directly_affected_modules,
        &impact.transitively_affected_modules,
        detected_languages,
        package_managers,
    );

    ImpactedTestsReport {
        tier1_modified_tests: tier1.into_iter().collect(),
        tier2_direct_tests: tier2.into_iter().collect(),
        tier3_transitive_tests: tier3.into_iter().collect(),
        all_selected_tests: all_selected_vec,
        uncovered_changed_files: impact.changed_files_without_tests,
        recommended_commands,
    }
}

/// Synthesizes deterministic test runner commands for the selected test files and affected modules.
#[must_use]
pub fn synthesize_test_commands(
    selected_tests: &[String],
    direct_modules: &[String],
    transitive_modules: &[String],
    detected_languages: &[String],
    package_managers: &[String],
) -> Vec<String> {
    let mut commands = Vec::new();

    let has_rust = detected_languages.iter().any(|l| l == "rust")
        || selected_tests.iter().any(|t| t.ends_with(".rs"))
        || direct_modules.iter().any(|m| m.starts_with("crates/"));

    if has_rust {
        let mut crates = BTreeSet::new();
        for m in direct_modules.iter().chain(transitive_modules.iter()) {
            if let Some(rest) = m.strip_prefix("crates/") {
                let crate_name = rest.split('/').next().unwrap_or(rest);
                if !crate_name.is_empty() {
                    crates.insert(crate_name.to_string());
                }
            }
        }
        if crates.is_empty() {
            commands.push("cargo test --workspace".to_string());
        } else {
            let flags: Vec<String> = crates.iter().map(|c| format!("--package {c}")).collect();
            commands.push(format!("cargo test {}", flags.join(" ")));
        }
    }

    let ts_tests: Vec<&str> = selected_tests
        .iter()
        .filter(|t| {
            t.ends_with(".ts") || t.ends_with(".tsx") || t.ends_with(".js") || t.ends_with(".jsx")
        })
        .map(String::as_str)
        .collect();
    if !ts_tests.is_empty() {
        let runner_prefix = if package_managers.iter().any(|p| p == "pnpm") {
            "pnpm vitest run"
        } else {
            "npx vitest run"
        };
        commands.push(format!("{runner_prefix} {}", ts_tests.join(" ")));
    }

    let py_tests: Vec<&str> = selected_tests
        .iter()
        .filter(|t| t.ends_with(".py"))
        .map(String::as_str)
        .collect();
    if !py_tests.is_empty() {
        commands.push(format!("pytest {}", py_tests.join(" ")));
    }

    let go_tests: Vec<&str> = selected_tests
        .iter()
        .filter(|t| t.ends_with(".go"))
        .map(String::as_str)
        .collect();
    if !go_tests.is_empty() {
        let mut pkgs = BTreeSet::new();
        for t in go_tests {
            let dir = Path::new(t)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or(".");
            pkgs.insert(format!("./{dir}"));
        }
        commands.push(format!(
            "go test {}",
            pkgs.into_iter().collect::<Vec<_>>().join(" ")
        ));
    }

    let php_tests: Vec<&str> = selected_tests
        .iter()
        .filter(|t| t.ends_with(".php"))
        .map(String::as_str)
        .collect();
    if !php_tests.is_empty() {
        commands.push(format!("vendor/bin/phpunit {}", php_tests.join(" ")));
    }

    commands
}

fn is_test_like(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.contains("_test.")
        || lower.starts_with("tests/")
        || lower.contains("/tests/")
        || lower.contains("/__tests__/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_direct_and_transitive_tests_and_builds_commands() {
        let modules = vec![
            Module {
                id: "src/db".to_string(),
                name: "db".to_string(),
                root_path: "src/db".to_string(),
                files: vec!["src/db/client.ts".to_string()],
                imports: vec![],
                exports: vec!["query".to_string()],
            },
            Module {
                id: "src/services".to_string(),
                name: "services".to_string(),
                root_path: "src/services".to_string(),
                files: vec![
                    "src/services/userService.ts".to_string(),
                    "src/services/userService.test.ts".to_string(),
                ],
                imports: vec!["src/db".to_string()],
                exports: vec!["getUser".to_string()],
            },
        ];

        let file_edges = vec![
            FileImportEdge {
                from_file: "src/services/userService.ts".to_string(),
                to_file: "src/db/client.ts".to_string(),
                from_module: "src/services".to_string(),
                to_module: "src/db".to_string(),
                raw_specifier: "../db/client".to_string(),
                line: 2,
                column: 24,
            },
            FileImportEdge {
                from_file: "src/services/userService.test.ts".to_string(),
                to_file: "src/services/userService.ts".to_string(),
                from_module: "src/services".to_string(),
                to_module: "src/services".to_string(),
                raw_specifier: "./userService".to_string(),
                line: 1,
                column: 25,
            },
        ];

        let changes = vec![FileChange {
            path: "src/db/client.ts".to_string(),
            added_lines: 3,
            deleted_lines: 1,
            is_new: false,
            is_deleted: false,
        }];

        let mut test_files = BTreeSet::new();
        test_files.insert("src/services/userService.test.ts".to_string());

        let report = compute_impacted_tests(
            &changes,
            &modules,
            &file_edges,
            &test_files,
            &["typescript".to_string()],
            &["pnpm".to_string()],
        );

        assert_eq!(
            report.tier3_transitive_tests,
            vec!["src/services/userService.test.ts"]
        );
        assert_eq!(
            report.all_selected_tests,
            vec!["src/services/userService.test.ts"]
        );
        assert_eq!(
            report.recommended_commands,
            vec!["pnpm vitest run src/services/userService.test.ts"]
        );
    }
}
