//! Matcher evaluating prohibited dependencies or version wildcard patterns.

use forge_core::{Finding, Repository};

use crate::declarative::DeclarativeRule;
use crate::matchers::{glob_match, slug_id};

/// Evaluates a `dependency_restriction` declarative rule against `repo`.
pub fn evaluate(rule: &DeclarativeRule, repo: &Repository) -> Vec<Finding> {
    let mut findings = Vec::new();

    if rule.matcher.disallowed_packages.is_empty() {
        return findings;
    }

    let checks_wildcard_version = rule.matcher.disallowed_packages.iter().any(|p| {
        p == "*"
            || p.eq_ignore_ascii_case("wildcard")
            || p.eq_ignore_ascii_case("wildcard-version")
            || p.eq_ignore_ascii_case("wildcards")
    });

    for dep in &repo.dependencies {
        let is_wildcard_req = dep.version_req == "*"
            || dep.version_req.eq_ignore_ascii_case("latest")
            || dep.version_req == "^*"
            || dep.version_req == "~*";

        if checks_wildcard_version && is_wildcard_req {
            findings.push(Finding {
                id: format!(
                    "DECL-DEP-WILDCARD-{}-{}",
                    slug_id(&rule.id),
                    slug_id(&dep.name)
                ),
                category: rule.category.as_str().to_string(),
                severity: rule.severity,
                confidence: rule.confidence,
                title: format!(
                    "Dependency '{}' uses unpinned or wildcard version '{}'",
                    dep.name, dep.version_req
                ),
                description: format!(
                    "{}: dependency '{}' specifies unpinned wildcard version '{}'.",
                    rule.name, dep.name, dep.version_req
                ),
                evidence: format!(
                    "ecosystem: {}, version_req: '{}', is_dev: {}",
                    dep.ecosystem, dep.version_req, dep.is_dev
                ),
                location: None,
                recommendation: if rule.remediation.is_empty() {
                    format!(
                        "Pin dependency '{}' to an exact or semver-bounded version.",
                        dep.name
                    )
                } else {
                    rule.remediation.clone()
                },
                rule_id: rule.id.clone(),
            });
            continue;
        }

        let is_disallowed = rule.matcher.disallowed_packages.iter().any(|disallowed| {
            if disallowed == "*" || disallowed.eq_ignore_ascii_case("wildcard-version") {
                return false;
            }
            if disallowed.contains('*') {
                glob_match(disallowed, &dep.name)
            } else {
                disallowed.eq_ignore_ascii_case(&dep.name)
            }
        });

        if is_disallowed {
            findings.push(Finding {
                id: format!(
                    "DECL-DEP-PROHIBITED-{}-{}",
                    slug_id(&rule.id),
                    slug_id(&dep.name)
                ),
                category: rule.category.as_str().to_string(),
                severity: rule.severity,
                confidence: rule.confidence,
                title: format!("Prohibited dependency '{}' detected", dep.name),
                description: format!(
                    "{}: dependency '{}' (version '{}') is prohibited by policy.",
                    rule.name, dep.name, dep.version_req
                ),
                evidence: format!(
                    "ecosystem: {}, version_req: '{}', is_dev: {}",
                    dep.ecosystem, dep.version_req, dep.is_dev
                ),
                location: None,
                recommendation: if rule.remediation.is_empty() {
                    format!(
                        "Remove dependency '{}' or migrate to an approved alternative.",
                        dep.name
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
    use forge_core::{Confidence, Dependency, Severity};

    #[test]
    fn detects_prohibited_packages_and_wildcards() {
        let repo = Repository {
            root_path: "/repo".to_string(),
            name: "test-repo".to_string(),
            detected_languages: vec!["typescript".to_string()],
            detected_frameworks: Vec::new(),
            package_managers: vec!["pnpm".to_string()],
            files: Vec::new(),
            modules: Vec::new(),
            dependencies: vec![
                Dependency {
                    name: "moment".to_string(),
                    version_req: "^2.29.4".to_string(),
                    resolved_version: Some("2.29.4".to_string()),
                    ecosystem: "npm".to_string(),
                    is_dev: false,
                    is_direct: true,
                },
                Dependency {
                    name: "lodash".to_string(),
                    version_req: "*".to_string(),
                    resolved_version: None,
                    ecosystem: "npm".to_string(),
                    is_dev: false,
                    is_direct: true,
                },
                Dependency {
                    name: "date-fns".to_string(),
                    version_req: "^3.0.0".to_string(),
                    resolved_version: Some("3.0.0".to_string()),
                    ecosystem: "npm".to_string(),
                    is_dev: false,
                    is_direct: true,
                },
            ],
            findings: Vec::new(),
            health: None,
        };

        let prohibited_rule = DeclarativeRule {
            id: "deps/no-moment".to_string(),
            name: "No Moment.js".to_string(),
            category: RuleCategory::Dependencies,
            severity: Severity::Medium,
            confidence: Confidence::Certain,
            description: "Use date-fns or temporal instead of moment".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::DependencyRestriction,
                disallowed_packages: vec!["moment".to_string()],
                ..Default::default()
            },
            remediation: "Migrate from moment to date-fns".to_string(),
            source: RuleSource::Builtin("deps".to_string()),
            enabled: true,
        };

        let findings = evaluate(&prohibited_rule, &repo);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].id, "DECL-DEP-PROHIBITED-DEPS_NO_MOMENT-MOMENT");

        let wildcard_rule = DeclarativeRule {
            id: "deps/no-wildcards".to_string(),
            name: "No Wildcard Versions".to_string(),
            category: RuleCategory::Dependencies,
            severity: Severity::High,
            confidence: Confidence::Certain,
            description: "Do not use * versions".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::DependencyRestriction,
                disallowed_packages: vec!["wildcard-version".to_string()],
                ..Default::default()
            },
            remediation: "Pin exact semver version".to_string(),
            source: RuleSource::Builtin("deps".to_string()),
            enabled: true,
        };

        let wildcard_findings = evaluate(&wildcard_rule, &repo);
        assert_eq!(wildcard_findings.len(), 1);
        assert_eq!(
            wildcard_findings[0].id,
            "DECL-DEP-WILDCARD-DEPS_NO_WILDCARDS-LODASH"
        );
    }
}
