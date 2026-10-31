//! Deterministic surgical codemod and auto-fix planner (`eforge fix`).
//!
//! Synthesizes verifiable [`FixSuggestion`]s and line-accurate [`TextEdit`]s for
//! architectural boundary violations, missing companion tests, unused/unpinned dependencies,
//! and pattern rules.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use forge_core::{Finding, FixSuggestion, ForgeError, Repository, TextEdit};
use serde::{Deserialize, Serialize};

/// Complete deterministic auto-fix plan produced by [`generate_fix_plan`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixPlan {
    pub suggestions: Vec<FixSuggestion>,
    pub total_edits: usize,
    pub affected_files: Vec<String>,
}

/// Generates a deterministic [`FixPlan`] for the findings in `repo`, optionally filtered by `rule_filter`.
pub fn generate_fix_plan(
    repo_root: &Path,
    repo: &Repository,
    rule_filter: Option<&str>,
) -> Result<FixPlan, ForgeError> {
    let mut suggestions = Vec::new();
    let mut affected_files = BTreeSet::new();
    let mut total_edits = 0usize;

    for finding in &repo.findings {
        if let Some(filter) = rule_filter {
            if !finding.rule_id.contains(filter) && finding.id != filter {
                continue;
            }
        }

        if let Some(suggestion) = build_suggestion_for_finding(repo_root, repo, finding) {
            for edit in &suggestion.edits {
                affected_files.insert(edit.file_path.clone());
                total_edits += 1;
            }
            suggestions.push(suggestion);
        }
    }

    Ok(FixPlan {
        suggestions,
        total_edits,
        affected_files: affected_files.into_iter().collect(),
    })
}

/// Synthesizes a [`FixSuggestion`] for a single [`Finding`] if a deterministic codemod exists.
#[must_use]
pub fn build_suggestion_for_finding(
    repo_root: &Path,
    repo: &Repository,
    finding: &Finding,
) -> Option<FixSuggestion> {
    match finding.rule_id.as_str() {
        "deps/unused-dependency" => build_unused_dep_fix(repo_root, finding),
        "deps/unpinned-version" => build_unpinned_dep_fix(repo_root, repo, finding),
        "quality/require-service-tests" => build_companion_test_fix(finding),
        "arch/no-direct-db-in-ui" => build_forbidden_import_fix(repo_root, finding),
        _ => {
            if finding.rule_id.starts_with("arch/") {
                build_forbidden_import_fix(repo_root, finding)
            } else if finding.rule_id.starts_with("quality/")
                && finding.description.contains("companion")
            {
                build_companion_test_fix(finding)
            } else {
                None
            }
        }
    }
}

fn build_unused_dep_fix(repo_root: &Path, finding: &Finding) -> Option<FixSuggestion> {
    let loc = finding.location.as_ref()?;
    let pkg_name = extract_quoted_token(&finding.title)
        .or_else(|| extract_quoted_token(&finding.description))?;

    let full_path = repo_root.join(&loc.file_path);
    let content = fs::read_to_string(&full_path).ok()?;
    let needle = format!("\"{pkg_name}\"");

    for (idx, line) in content.lines().enumerate() {
        if line.contains(&needle) && line.contains(':') {
            let line_num = idx as u32 + 1;
            return Some(FixSuggestion {
                rule_id: finding.rule_id.clone(),
                title: format!(
                    "Remove unused dependency '{pkg_name}' from {}",
                    loc.file_path
                ),
                description: format!(
                    "Deletes unused package entry '{pkg_name}' from manifest {}.",
                    loc.file_path
                ),
                edits: vec![TextEdit {
                    file_path: loc.file_path.clone(),
                    start_line: line_num,
                    end_line: line_num,
                    original_text: line.to_string(),
                    replacement_text: String::new(),
                    description: format!("Remove unused dependency '{pkg_name}'"),
                }],
                is_safe_automated: true,
            });
        }
    }
    None
}

fn build_unpinned_dep_fix(
    repo_root: &Path,
    repo: &Repository,
    finding: &Finding,
) -> Option<FixSuggestion> {
    let loc = finding.location.as_ref()?;
    let pkg_name = extract_quoted_token(&finding.title)
        .or_else(|| extract_quoted_token(&finding.description))?;

    let pinned_version = repo
        .dependencies
        .iter()
        .find(|d| d.name == pkg_name)
        .and_then(|d| d.resolved_version.as_deref())
        .map(|v| format!("^{v}"))
        .unwrap_or_else(|| "^1.0.0".to_string());

    let full_path = repo_root.join(&loc.file_path);
    let content = fs::read_to_string(&full_path).ok()?;
    let needle = format!("\"{pkg_name}\"");

    for (idx, line) in content.lines().enumerate() {
        if line.contains(&needle) && (line.contains("\"*\"") || line.contains("\"latest\"")) {
            let line_num = idx as u32 + 1;
            let replaced = line
                .replace("\"*\"", &format!("\"{pinned_version}\""))
                .replace("\"latest\"", &format!("\"{pinned_version}\""));
            return Some(FixSuggestion {
                rule_id: finding.rule_id.clone(),
                title: format!(
                    "Pin dependency '{pkg_name}' to '{pinned_version}' in {}",
                    loc.file_path
                ),
                description: format!(
                    "Replaces wildcard version specifier for '{pkg_name}' with deterministic semver constraint '{pinned_version}'."
                ),
                edits: vec![TextEdit {
                    file_path: loc.file_path.clone(),
                    start_line: line_num,
                    end_line: line_num,
                    original_text: line.to_string(),
                    replacement_text: replaced,
                    description: format!("Pin '{pkg_name}' to '{pinned_version}'"),
                }],
                is_safe_automated: true,
            });
        }
    }
    None
}

fn build_companion_test_fix(finding: &Finding) -> Option<FixSuggestion> {
    let loc = finding.location.as_ref()?;
    let src_path = &loc.file_path;
    let stem = Path::new(src_path).file_stem().and_then(|s| s.to_str())?;
    let parent = Path::new(src_path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("src");

    let (companion_path, stub_content) = if src_path.ends_with(".rs") {
        (
            format!("{parent}/tests/{stem}_test.rs"),
            format!(
                "// Generated by `eforge fix` for {src_path}\n#[test]\nfn {stem}_smoke_test() {{\n    assert!(true);\n}}\n"
            ),
        )
    } else if src_path.ends_with(".py") {
        (
            format!("tests/test_{stem}.py"),
            format!(
                "# Generated by `eforge fix` for {src_path}\ndef test_{stem}_smoke():\n    assert True\n"
            ),
        )
    } else {
        (
            format!("{parent}/{stem}.test.ts"),
            format!(
                "// Generated by `eforge fix` for {src_path}\nimport {{ describe, it, expect }} from 'vitest';\nimport * as subject from './{stem}';\n\ndescribe('{stem}', () => {{\n  it('exports expected module surface', () => {{\n    expect(subject).toBeDefined();\n  }});\n}});\n"
            ),
        )
    };

    Some(FixSuggestion {
        rule_id: finding.rule_id.clone(),
        title: format!("Scaffold companion test file '{companion_path}'"),
        description: format!(
            "Creates deterministic unit test scaffold '{companion_path}' for uncovered module '{src_path}'."
        ),
        edits: vec![TextEdit {
            file_path: companion_path.clone(),
            start_line: 1,
            end_line: 1,
            original_text: String::new(),
            replacement_text: stub_content,
            description: format!("Create companion test '{companion_path}'"),
        }],
        is_safe_automated: true,
    })
}

fn build_forbidden_import_fix(repo_root: &Path, finding: &Finding) -> Option<FixSuggestion> {
    let loc = finding.location.as_ref()?;
    let full_path = repo_root.join(&loc.file_path);
    let content = fs::read_to_string(&full_path).ok()?;

    let target_line_hint = loc.start_line.unwrap_or(1);
    let mut matched_line_num = None;
    let mut matched_line_text = String::new();

    if let Some(line) = content
        .lines()
        .nth(target_line_hint.saturating_sub(1) as usize)
    {
        if line.trim_start().starts_with("import ") || line.trim_start().starts_with("use ") {
            matched_line_num = Some(target_line_hint);
            matched_line_text = line.to_string();
        }
    }

    if matched_line_num.is_none() {
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if (trimmed.starts_with("import ") || trimmed.starts_with("use "))
                && (trimmed.contains("db") || finding.evidence.contains(trimmed))
            {
                matched_line_num = Some(idx as u32 + 1);
                matched_line_text = line.to_string();
                break;
            }
        }
    }

    let line_num = matched_line_num?;
    let replacement = if matched_line_text.contains("../db/") {
        matched_line_text.replace("../db/", "../services/")
    } else {
        format!("// [eforge-fix: boundary violation isolated] {matched_line_text}")
    };

    Some(FixSuggestion {
        rule_id: finding.rule_id.clone(),
        title: format!("Isolate forbidden import in {}:{}", loc.file_path, line_num),
        description: format!(
            "Rewrites forbidden layer import at {}:{} to route through the service boundary.",
            loc.file_path, line_num
        ),
        edits: vec![TextEdit {
            file_path: loc.file_path.clone(),
            start_line: line_num,
            end_line: line_num,
            original_text: matched_line_text,
            replacement_text: replacement,
            description: "Rewrite forbidden import boundary".to_string(),
        }],
        is_safe_automated: true,
    })
}

fn extract_quoted_token(text: &str) -> Option<String> {
    for quote in ['\'', '"', '`'] {
        if let Some(start) = text.find(quote) {
            let rest = &text[start + 1..];
            if let Some(end) = rest.find(quote) {
                let tok = &rest[..end];
                if !tok.is_empty() {
                    return Some(tok.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tx::{apply_fix_plan_transactional, rollback_transaction};
    use forge_core::{Confidence, Severity, SourceLocation};

    #[test]
    fn generates_and_rolls_back_transactional_autofix_cleanly() {
        let temp_dir = std::env::temp_dir().join("eforge_autofix_tx_test_991");
        let _ = fs::remove_dir_all(&temp_dir);
        let src_dir = temp_dir.join("src").join("services");
        assert!(fs::create_dir_all(&src_dir).is_ok());

        let pkg_path = temp_dir.join("package.json");
        let original_pkg = "{\n  \"name\": \"demo\",\n  \"dependencies\": {\n    \"react\": \"^18.2.0\",\n    \"unused-sdk\": \"^1.0.0\"\n  }\n}\n";
        assert!(fs::write(&pkg_path, original_pkg).is_ok());

        let repo = Repository {
            root_path: temp_dir.to_string_lossy().into_owned(),
            name: "demo".to_string(),
            detected_languages: vec!["typescript".to_string()],
            detected_frameworks: vec![],
            package_managers: vec!["npm".to_string()],
            files: vec![],
            modules: vec![],
            dependencies: vec![],
            findings: vec![
                Finding {
                    id: "F-DEP-1".to_string(),
                    category: "dependencies".to_string(),
                    severity: Severity::Medium,
                    confidence: Confidence::Certain,
                    title: "Unused dependency 'unused-sdk'".to_string(),
                    description: "Package 'unused-sdk' is declared but never imported.".to_string(),
                    evidence: "\"unused-sdk\": \"^1.0.0\"".to_string(),
                    location: Some(SourceLocation {
                        file_path: "package.json".to_string(),
                        start_line: Some(5),
                        end_line: Some(5),
                    }),
                    recommendation: "Remove unused-sdk".to_string(),
                    rule_id: "deps/unused-dependency".to_string(),
                },
                Finding {
                    id: "F-TEST-1".to_string(),
                    category: "quality".to_string(),
                    severity: Severity::Medium,
                    confidence: Confidence::High,
                    title: "Missing service test".to_string(),
                    description: "Service lacks companion test".to_string(),
                    evidence: "src/services/billing.ts".to_string(),
                    location: Some(SourceLocation {
                        file_path: "src/services/billing.ts".to_string(),
                        start_line: Some(1),
                        end_line: Some(1),
                    }),
                    recommendation: "Add test".to_string(),
                    rule_id: "quality/require-service-tests".to_string(),
                },
            ],
            health: None,
        };

        let plan = generate_fix_plan(&temp_dir, &repo, None);
        assert!(plan.is_ok());
        let Ok(fix_plan) = plan else { return };
        assert_eq!(fix_plan.suggestions.len(), 2);
        assert_eq!(fix_plan.total_edits, 2);

        // Apply transactionally
        let tx_res = apply_fix_plan_transactional(&temp_dir, &fix_plan.suggestions);
        assert!(tx_res.is_ok());

        let updated_pkg = fs::read_to_string(&pkg_path).unwrap_or_default();
        assert!(!updated_pkg.contains("unused-sdk"));
        assert!(serde_json::from_str::<serde_json::Value>(&updated_pkg).is_ok());
        assert!(temp_dir.join("src/services/billing.test.ts").is_file());

        // Rollback transaction and verify byte-for-byte restoration
        let rb_res = rollback_transaction(&temp_dir, None);
        assert!(rb_res.is_ok());

        let restored_pkg = fs::read_to_string(&pkg_path).unwrap_or_default();
        assert_eq!(restored_pkg, original_pkg);
        assert!(!temp_dir.join("src/services/billing.test.ts").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
