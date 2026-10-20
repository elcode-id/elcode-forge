//! Matcher scanning source file content for forbidden tokens or antipatterns using regular expressions.

use std::fs;
use std::path::Path;

use forge_core::path_guard::resolve_safe_path;
use forge_core::{Finding, Repository, SourceLocation};
use regex::Regex;

use crate::declarative::DeclarativeRule;
use crate::matchers::{matches_any_pattern, slug_id};

/// Maximum finding reports per individual file to prevent output flood.
const MAX_MATCHES_PER_FILE: usize = 10;

/// Evaluates a `regex_pattern` declarative rule against `repo`.
pub fn evaluate(rule: &DeclarativeRule, repo: &Repository) -> Vec<Finding> {
    let mut findings = Vec::new();

    let Some(pattern_str) = &rule.matcher.pattern else {
        return findings;
    };

    if pattern_str.is_empty() {
        return findings;
    }

    let Ok(re) = Regex::new(pattern_str) else {
        return findings;
    };

    let repo_root = Path::new(&repo.root_path);

    for file in &repo.files {
        if !matches_any_pattern(&file.relative_path, &rule.matcher.files) {
            continue;
        }

        let Ok(safe_path) = resolve_safe_path(repo_root, Path::new(&file.relative_path)) else {
            continue;
        };

        let Ok(content) = fs::read_to_string(&safe_path) else {
            continue;
        };

        let mut matches_in_file = 0;

        for (line_idx, line) in content.lines().enumerate() {
            let line_no = (line_idx + 1) as u32;

            if re.is_match(line) {
                matches_in_file += 1;
                let trimmed = line.trim();

                findings.push(Finding {
                    id: format!(
                        "DECL-REGEX-{}-{}-L{}",
                        slug_id(&rule.id),
                        slug_id(&file.relative_path),
                        line_no
                    ),
                    category: rule.category.as_str().to_string(),
                    severity: rule.severity,
                    confidence: rule.confidence,
                    title: format!(
                        "Forbidden pattern '{}' matched in '{}'",
                        pattern_str, file.relative_path
                    ),
                    description: format!(
                        "{}: pattern '{}' matched in '{}' at line {line_no}.",
                        rule.name, pattern_str, file.relative_path
                    ),
                    evidence: format!("line {line_no}: {trimmed}"),
                    location: Some(SourceLocation {
                        file_path: file.relative_path.clone(),
                        start_line: Some(line_no),
                        end_line: Some(line_no),
                    }),
                    recommendation: if rule.remediation.is_empty() {
                        format!("Remove or refactor matches of pattern '{}'.", pattern_str)
                    } else {
                        rule.remediation.clone()
                    },
                    rule_id: rule.id.clone(),
                });

                if matches_in_file >= MAX_MATCHES_PER_FILE {
                    break;
                }
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
    use std::fs;

    #[test]
    fn scans_and_reports_regex_pattern_matches() {
        let temp_dir =
            std::env::temp_dir().join(format!("forge_test_regex_{}", std::process::id()));
        if fs::create_dir_all(&temp_dir).is_err() {
            return;
        }

        let file_path = temp_dir.join("app.ts");
        let content = "const x = 1;\nconsole.log(x);\neval('2 + 2');\n";
        if fs::write(&file_path, content).is_err() {
            return;
        }

        let repo = Repository {
            root_path: temp_dir.to_string_lossy().to_string(),
            name: "test-repo".to_string(),
            detected_languages: vec!["typescript".to_string()],
            detected_frameworks: Vec::new(),
            package_managers: Vec::new(),
            files: vec![FileNode {
                relative_path: "app.ts".to_string(),
                language: Some("typescript".to_string()),
                size_bytes: content.len() as u64,
                line_count: 3,
                content_hash: "hash".to_string(),
                is_test: false,
            }],
            modules: Vec::new(),
            dependencies: Vec::new(),
            findings: Vec::new(),
            health: None,
        };

        let rule = DeclarativeRule {
            id: "security/no-eval".to_string(),
            name: "No Eval".to_string(),
            category: RuleCategory::Security,
            severity: Severity::Critical,
            confidence: Confidence::Certain,
            description: "Disallow eval".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::RegexPattern,
                files: vec!["**/*.ts".to_string()],
                pattern: Some(r"eval\(".to_string()),
                ..Default::default()
            },
            remediation: "Use structured code".to_string(),
            source: RuleSource::Builtin("test".to_string()),
            enabled: true,
        };

        let findings = evaluate(&rule, &repo);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Critical);
        if let Some(loc) = &findings[0].location {
            assert_eq!(loc.start_line, Some(3));
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
