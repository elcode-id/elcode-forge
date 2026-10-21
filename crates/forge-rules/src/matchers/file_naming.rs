//! Matcher enforcing file naming conventions (e.g. `*Controller.ts`, kebab-case, PascalCase).

use std::path::Path;

use forge_core::{Finding, Repository, SourceLocation};

use crate::declarative::DeclarativeRule;
use crate::matchers::{glob_match, matches_any_pattern, slug_id};

/// Evaluates a `file_naming` declarative rule against `repo`.
pub fn evaluate(rule: &DeclarativeRule, repo: &Repository) -> Vec<Finding> {
    let mut findings = Vec::new();
    let Some(ref convention) = rule.matcher.convention else {
        return findings;
    };

    for file in &repo.files {
        if !matches_any_pattern(&file.relative_path, &rule.matcher.files) {
            continue;
        }

        let path = Path::new(&file.relative_path);
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();

        let is_valid = match convention.as_str() {
            "kebab-case" => is_kebab_case(file_stem),
            "camelCase" => is_camel_case(file_stem),
            "PascalCase" => is_pascal_case(file_stem),
            "snake_case" => is_snake_case(file_stem),
            pattern if pattern.contains('*') => glob_match(pattern, file_name),
            exact => file_name == exact || file_stem == exact,
        };

        if !is_valid {
            findings.push(Finding {
                id: format!(
                    "DECL-NAMING-{}-{}",
                    slug_id(&rule.id),
                    slug_id(&file.relative_path)
                ),
                category: rule.category.as_str().to_string(),
                severity: rule.severity,
                confidence: rule.confidence,
                title: format!(
                    "File '{}' violates naming convention '{}'",
                    file.relative_path, convention
                ),
                description: format!(
                    "{}: file '{}' does not comply with naming rule '{}'",
                    rule.name, file.relative_path, convention
                ),
                evidence: format!("filename '{}' does not match '{}'", file_name, convention),
                location: Some(SourceLocation {
                    file_path: file.relative_path.clone(),
                    start_line: Some(1),
                    end_line: None,
                }),
                recommendation: if rule.remediation.is_empty() {
                    format!(
                        "Rename '{}' to adhere to convention '{}'",
                        file_name, convention
                    )
                } else {
                    rule.remediation.clone()
                },
                rule_id: rule.id.clone(),
            });
        }
    }

    findings
}

fn is_kebab_case(s: &str) -> bool {
    if s.is_empty() || s.starts_with('-') || s.ends_with('-') {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_snake_case(s: &str) -> bool {
    if s.is_empty() || s.starts_with('_') || s.ends_with('_') {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn is_camel_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.chars().next().unwrap_or(' ');
    if !first.is_ascii_lowercase() {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric())
}

fn is_pascal_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.chars().next().unwrap_or(' ');
    if !first.is_ascii_uppercase() {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declarative::{MatcherKind, RuleCategory, RuleMatcher, RuleSource};
    use forge_core::{Confidence, FileNode, Severity};

    #[test]
    fn enforces_file_naming_conventions() {
        let rule = DeclarativeRule {
            id: "quality/controller-suffix".to_string(),
            name: "Controller Suffix".to_string(),
            category: RuleCategory::Quality,
            severity: Severity::Low,
            confidence: Confidence::Certain,
            description: "Controllers must end in Controller.ts".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::FileNaming,
                files: vec!["src/controllers/**".to_string()],
                convention: Some("*Controller.ts".to_string()),
                ..Default::default()
            },
            remediation: "Rename to *Controller.ts".to_string(),
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
                    relative_path: "src/controllers/UserController.ts".to_string(),
                    language: Some("typescript".to_string()),
                    size_bytes: 100,
                    line_count: 10,
                    content_hash: "h1".to_string(),
                    is_test: false,
                },
                FileNode {
                    relative_path: "src/controllers/auth_handler.ts".to_string(),
                    language: Some("typescript".to_string()),
                    size_bytes: 100,
                    line_count: 10,
                    content_hash: "h2".to_string(),
                    is_test: false,
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
            Some(&"src/controllers/auth_handler.ts".to_string())
        );
    }
}
