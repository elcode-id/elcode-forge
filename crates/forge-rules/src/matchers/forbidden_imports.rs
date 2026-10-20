//! Matcher evaluating forbidden import relationships between modules or packages.

use forge_core::{Finding, Repository, SourceLocation};

use crate::declarative::DeclarativeRule;
use crate::matchers::{matches_any_pattern, slug_id};

/// Evaluates a `forbidden_imports` declarative rule against `repo`.
pub fn evaluate(rule: &DeclarativeRule, repo: &Repository) -> Vec<Finding> {
    let mut findings = Vec::new();

    for module in &repo.modules {
        let matches_source = if rule.matcher.files.is_empty() {
            true
        } else {
            matches_any_pattern(&module.root_path, &rule.matcher.files)
                || module
                    .files
                    .iter()
                    .any(|f| matches_any_pattern(f, &rule.matcher.files))
        };

        if !matches_source {
            continue;
        }

        for imported in &module.imports {
            let is_forbidden = matches_any_pattern(imported, &rule.matcher.forbidden);
            if !is_forbidden {
                continue;
            }

            let first_file = module
                .files
                .iter()
                .find(|f| matches_any_pattern(f, &rule.matcher.files))
                .cloned()
                .or_else(|| module.files.first().cloned());

            findings.push(Finding {
                id: format!(
                    "DECL-IMPORT-{}-{}",
                    slug_id(&rule.id),
                    slug_id(&format!("{}_{}", module.id, imported))
                ),
                category: rule.category.as_str().to_string(),
                severity: rule.severity,
                confidence: rule.confidence,
                title: format!(
                    "Module '{}' imports forbidden target '{}'",
                    module.id, imported
                ),
                description: format!(
                    "{}: module '{}' imports '{}', violating boundary rule '{}'",
                    rule.name, module.id, imported, rule.id
                ),
                evidence: format!("{} -> {}", module.id, imported),
                location: first_file.map(|file_path| SourceLocation {
                    file_path,
                    start_line: None,
                    end_line: None,
                }),
                recommendation: if rule.remediation.is_empty() {
                    format!(
                        "Remove or decouple the import of '{}' from '{}'",
                        imported, module.id
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declarative::{MatcherKind, RuleCategory, RuleMatcher, RuleSource};
    use forge_core::{Confidence, Module, Severity};

    #[test]
    fn detects_forbidden_imports_between_modules() {
        let rule = DeclarativeRule {
            id: "arch/no-db-in-ui".to_string(),
            name: "No DB in UI".to_string(),
            category: RuleCategory::Architecture,
            severity: Severity::High,
            confidence: Confidence::Certain,
            description: "UI modules cannot import database modules".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::ForbiddenImports,
                files: vec!["src/ui/**".to_string()],
                forbidden: vec!["src/db/**".to_string(), "@prisma/client".to_string()],
                ..Default::default()
            },
            remediation: "Use services".to_string(),
            source: RuleSource::Builtin("test".to_string()),
            enabled: true,
        };

        let repo = Repository {
            root_path: ".".to_string(),
            name: "test-repo".to_string(),
            detected_languages: vec!["typescript".to_string()],
            detected_frameworks: Vec::new(),
            package_managers: Vec::new(),
            files: Vec::new(),
            modules: vec![
                Module {
                    id: "ui".to_string(),
                    name: "ui".to_string(),
                    root_path: "src/ui".to_string(),
                    files: vec!["src/ui/Card.tsx".to_string()],
                    imports: vec!["src/db/client".to_string()],
                    exports: Vec::new(),
                },
                Module {
                    id: "services".to_string(),
                    name: "services".to_string(),
                    root_path: "src/services".to_string(),
                    files: vec!["src/services/user.ts".to_string()],
                    imports: vec!["src/db/client".to_string()],
                    exports: Vec::new(),
                },
            ],
            dependencies: Vec::new(),
            findings: Vec::new(),
            health: None,
        };

        let findings = evaluate(&rule, &repo);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "arch/no-db-in-ui");
        assert_eq!(findings[0].evidence, "ui -> src/db/client");
    }
}
