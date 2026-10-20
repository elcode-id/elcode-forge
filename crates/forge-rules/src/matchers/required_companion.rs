//! Matcher verifying required companion files (e.g. test files for services/controllers).

use std::path::Path;

use forge_core::{Finding, Repository, SourceLocation};

use crate::declarative::DeclarativeRule;
use crate::matchers::{matches_any_pattern, slug_id};

/// Evaluates a `required_companion` declarative rule against `repo`.
pub fn evaluate(rule: &DeclarativeRule, repo: &Repository) -> Vec<Finding> {
    let mut findings = Vec::new();
    let Some(ref companion_template) = rule.matcher.companion else {
        return findings;
    };

    for file in &repo.files {
        if file.is_test {
            continue;
        }

        if !matches_any_pattern(&file.relative_path, &rule.matcher.files) {
            continue;
        }

        let path = Path::new(&file.relative_path);
        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let parent_dir = path
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or_default()
            .replace('\\', "/");
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();

        let companion_expected = companion_template
            .replace("{name}", file_stem)
            .replace("{stem}", file_stem)
            .replace("{base}", file_name)
            .replace("{dir}", &parent_dir)
            .replace("{ext}", ext)
            .replace('\\', "/");

        let companion_exists = repo.files.iter().any(|f| {
            let rel = f.relative_path.replace('\\', "/");
            if rel == companion_expected {
                return true;
            }
            if companion_template.contains('*') {
                return crate::matchers::glob_match(&companion_expected, &rel);
            }
            false
        });

        if !companion_exists {
            findings.push(Finding {
                id: format!(
                    "DECL-COMPANION-{}-{}",
                    slug_id(&rule.id),
                    slug_id(&file.relative_path)
                ),
                category: rule.category.as_str().to_string(),
                severity: rule.severity,
                confidence: rule.confidence,
                title: format!(
                    "File '{}' is missing required companion file",
                    file.relative_path
                ),
                description: format!(
                    "{}: file '{}' requires a companion matching '{}'",
                    rule.name, file.relative_path, companion_expected
                ),
                evidence: format!("expected companion: {}", companion_expected),
                location: Some(SourceLocation {
                    file_path: file.relative_path.clone(),
                    start_line: Some(1),
                    end_line: None,
                }),
                recommendation: if rule.remediation.is_empty() {
                    format!("Create companion file '{}'", companion_expected)
                } else {
                    rule.remediation.clone()
                },
                rule_id: rule.id.clone(),
            });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declarative::{MatcherKind, RuleCategory, RuleMatcher, RuleSource};
    use forge_core::{Confidence, FileNode, Severity};

    #[test]
    fn detects_missing_companion_test_file() {
        let rule = DeclarativeRule {
            id: "quality/require-service-tests".to_string(),
            name: "Require Service Tests".to_string(),
            category: RuleCategory::Quality,
            severity: Severity::Medium,
            confidence: Confidence::Certain,
            description: "Services must have matching tests".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::RequiredCompanion,
                files: vec!["src/services/**".to_string()],
                companion: Some("tests/{name}.test.ts".to_string()),
                ..Default::default()
            },
            remediation: "Create tests/{name}.test.ts".to_string(),
            source: RuleSource::Builtin("test".to_string()),
            enabled: true,
        };

        let repo = Repository {
            root_path: ".".to_string(),
            name: "test-repo".to_string(),
            detected_languages: vec!["typescript".to_string()],
            detected_frameworks: Vec::new(),
            package_managers: Vec::new(),
            files: vec![
                FileNode {
                    relative_path: "src/services/UserService.ts".to_string(),
                    language: Some("typescript".to_string()),
                    size_bytes: 100,
                    line_count: 10,
                    content_hash: "h1".to_string(),
                    is_test: false,
                },
                FileNode {
                    relative_path: "src/services/AuthService.ts".to_string(),
                    language: Some("typescript".to_string()),
                    size_bytes: 100,
                    line_count: 10,
                    content_hash: "h2".to_string(),
                    is_test: false,
                },
                FileNode {
                    relative_path: "tests/UserService.test.ts".to_string(),
                    language: Some("typescript".to_string()),
                    size_bytes: 50,
                    line_count: 5,
                    content_hash: "h3".to_string(),
                    is_test: true,
                },
            ],
            modules: Vec::new(),
            dependencies: Vec::new(),
            findings: Vec::new(),
            health: None,
        };

        let findings = evaluate(&rule, &repo);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].location.as_ref().map(|l| &l.file_path),
            Some(&"src/services/AuthService.ts".to_string())
        );
        assert_eq!(
            findings[0].evidence,
            "expected companion: tests/AuthService.test.ts"
        );
    }
}
