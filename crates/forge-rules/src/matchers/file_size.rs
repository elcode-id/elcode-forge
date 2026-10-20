//! Matcher enforcing maximum line count or byte limits on source files.

use forge_core::{Finding, Repository, SourceLocation};

use crate::declarative::DeclarativeRule;
use crate::matchers::{matches_any_pattern, slug_id};

/// Evaluates a `file_size` declarative rule against `repo`.
pub fn evaluate(rule: &DeclarativeRule, repo: &Repository) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file in &repo.files {
        if !matches_any_pattern(&file.relative_path, &rule.matcher.files) {
            continue;
        }

        if let Some(max_lines) = rule.matcher.max_lines {
            if file.line_count > max_lines {
                findings.push(Finding {
                    id: format!(
                        "DECL-LINES-{}-{}",
                        slug_id(&rule.id),
                        slug_id(&file.relative_path)
                    ),
                    category: rule.category.as_str().to_string(),
                    severity: rule.severity,
                    confidence: rule.confidence,
                    title: format!(
                        "File '{}' exceeds maximum line threshold ({} > {})",
                        file.relative_path, file.line_count, max_lines
                    ),
                    description: format!(
                        "{}: file '{}' has {} lines, exceeding limit of {}.",
                        rule.name, file.relative_path, file.line_count, max_lines
                    ),
                    evidence: format!("line_count = {}", file.line_count),
                    location: Some(SourceLocation {
                        file_path: file.relative_path.clone(),
                        start_line: Some(1),
                        end_line: None,
                    }),
                    recommendation: if rule.remediation.is_empty() {
                        "Split this file into smaller, single-responsibility modules.".to_string()
                    } else {
                        rule.remediation.clone()
                    },
                    rule_id: rule.id.clone(),
                });
            }
        }

        if let Some(max_bytes) = rule.matcher.max_bytes {
            if file.size_bytes > max_bytes {
                findings.push(Finding {
                    id: format!(
                        "DECL-BYTES-{}-{}",
                        slug_id(&rule.id),
                        slug_id(&file.relative_path)
                    ),
                    category: rule.category.as_str().to_string(),
                    severity: rule.severity,
                    confidence: rule.confidence,
                    title: format!(
                        "File '{}' exceeds maximum byte threshold ({} > {})",
                        file.relative_path, file.size_bytes, max_bytes
                    ),
                    description: format!(
                        "{}: file '{}' has {} bytes, exceeding limit of {}.",
                        rule.name, file.relative_path, file.size_bytes, max_bytes
                    ),
                    evidence: format!("size_bytes = {}", file.size_bytes),
                    location: Some(SourceLocation {
                        file_path: file.relative_path.clone(),
                        start_line: Some(1),
                        end_line: None,
                    }),
                    recommendation: if rule.remediation.is_empty() {
                        "Reduce file payload size or split into smaller assets.".to_string()
                    } else {
                        rule.remediation.clone()
                    },
                    rule_id: rule.id.clone(),
                });
            }
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
    fn enforces_file_size_limits() {
        let rule = DeclarativeRule {
            id: "quality/max-lines".to_string(),
            name: "Max Lines".to_string(),
            category: RuleCategory::Quality,
            severity: Severity::Low,
            confidence: Confidence::Certain,
            description: "Files must be under 50 lines".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::FileSize,
                files: vec!["src/**".to_string()],
                max_lines: Some(50),
                ..Default::default()
            },
            remediation: "Split file".to_string(),
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
                    relative_path: "src/small.ts".to_string(),
                    language: Some("typescript".to_string()),
                    size_bytes: 100,
                    line_count: 20,
                    content_hash: "h1".to_string(),
                    is_test: false,
                },
                FileNode {
                    relative_path: "src/large.ts".to_string(),
                    language: Some("typescript".to_string()),
                    size_bytes: 1000,
                    line_count: 120,
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
        assert_eq!(findings[0].evidence, "line_count = 120");
    }
}
