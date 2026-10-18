use std::collections::BTreeMap;

use forge_core::{Analyzer, Finding, ForgeError, Repository, RuleOverrideConfig, Severity};
use serde::{Deserialize, Serialize};

use crate::builtin::{
    check_max_file_lines, check_no_circular_dependencies, check_no_direct_db_in_ui,
    check_require_service_tests, RULE_MAX_FILE_LINES, RULE_NO_CIRCULAR_DEPS,
    RULE_NO_DIRECT_DB_IN_UI, RULE_REQUIRE_SERVICE_TESTS,
};

/// Declarative configuration for an individual rule inside [`RuleEngine`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleConfig {
    pub id: String,
    pub enabled: bool,
    pub severity: Severity,
    pub threshold: Option<u32>,
}

/// Deterministic rule evaluation engine.
#[derive(Debug, Clone)]
pub struct RuleEngine {
    rules: Vec<RuleConfig>,
}

impl RuleEngine {
    /// Creates a [`RuleEngine`] with the provided custom rule configurations.
    pub fn new(rules: Vec<RuleConfig>) -> Self {
        Self { rules }
    }

    /// Creates a [`RuleEngine`] initialized with ELCODE Forge's recommended default ruleset.
    pub fn with_recommended_defaults() -> Self {
        Self {
            rules: vec![
                RuleConfig {
                    id: RULE_NO_DIRECT_DB_IN_UI.to_string(),
                    enabled: true,
                    severity: Severity::High,
                    threshold: None,
                },
                RuleConfig {
                    id: RULE_NO_CIRCULAR_DEPS.to_string(),
                    enabled: true,
                    severity: Severity::High,
                    threshold: None,
                },
                RuleConfig {
                    id: RULE_REQUIRE_SERVICE_TESTS.to_string(),
                    enabled: true,
                    severity: Severity::Medium,
                    threshold: None,
                },
                RuleConfig {
                    id: RULE_MAX_FILE_LINES.to_string(),
                    enabled: true,
                    severity: Severity::Low,
                    threshold: Some(300),
                },
            ],
        }
    }

    /// Creates a [`RuleEngine`] initialized with recommended defaults and merged with user
    /// overrides from `eforge.config.yaml`.
    pub fn with_config_overrides(overrides: &BTreeMap<String, RuleOverrideConfig>) -> Self {
        let mut engine = Self::with_recommended_defaults();
        for rule in &mut engine.rules {
            if let Some(ov) = overrides.get(&rule.id) {
                if let Some(enabled) = ov.enabled {
                    rule.enabled = enabled;
                }
                if let Some(sev) = ov.severity {
                    rule.severity = sev;
                }
                if let Some(max_lines) = ov.max_lines {
                    rule.threshold = Some(max_lines);
                }
            }
        }
        engine
    }

    /// Returns the slice of configured rules.
    pub fn rules(&self) -> &[RuleConfig] {
        &self.rules
    }

    /// Evaluates all enabled rules against `repo` and returns deterministically sorted [`Finding`]s.
    pub fn evaluate(&self, repo: &Repository) -> Result<Vec<Finding>, ForgeError> {
        let mut findings = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            match rule.id.as_str() {
                RULE_NO_DIRECT_DB_IN_UI => {
                    findings.extend(check_no_direct_db_in_ui(repo, rule.severity));
                }
                RULE_NO_CIRCULAR_DEPS => {
                    findings.extend(check_no_circular_dependencies(repo, rule.severity));
                }
                RULE_REQUIRE_SERVICE_TESTS => {
                    findings.extend(check_require_service_tests(repo, rule.severity));
                }
                RULE_MAX_FILE_LINES => {
                    let threshold = rule.threshold.unwrap_or(300);
                    findings.extend(check_max_file_lines(repo, rule.severity, threshold));
                }
                _ => {}
            }
        }

        findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
        Ok(findings)
    }
}

impl Analyzer for RuleEngine {
    fn name(&self) -> &'static str {
        "forge-rules"
    }

    fn analyze(&self, repo: &Repository) -> Result<Vec<Finding>, ForgeError> {
        self.evaluate(repo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::{FileNode, Module};

    #[test]
    fn evaluates_all_four_builtin_rules_deterministically() {
        let repo = Repository {
            root_path: "/workspace/fixture-next-app".to_string(),
            name: "fixture-next-app".to_string(),
            detected_languages: vec!["typescript".to_string()],
            detected_frameworks: vec!["nextjs".to_string()],
            package_managers: vec!["npm".to_string()],
            files: vec![
                FileNode {
                    relative_path: "src/components/UserCard.tsx".to_string(),
                    language: Some("typescript".to_string()),
                    size_bytes: 512,
                    line_count: 350, // Exceeds 300 line threshold
                    content_hash: "h1".to_string(),
                    is_test: false,
                },
                FileNode {
                    relative_path: "src/services/userService.ts".to_string(),
                    language: Some("typescript".to_string()),
                    size_bytes: 256,
                    line_count: 25,
                    content_hash: "h2".to_string(),
                    is_test: false,
                },
                FileNode {
                    relative_path: "src/db/client.ts".to_string(),
                    language: Some("typescript".to_string()),
                    size_bytes: 256,
                    line_count: 20,
                    content_hash: "h3".to_string(),
                    is_test: false,
                },
            ],
            modules: vec![
                Module {
                    id: "src/components".to_string(),
                    name: "components".to_string(),
                    root_path: "src/components".to_string(),
                    files: vec!["src/components/UserCard.tsx".to_string()],
                    imports: vec!["src/db".to_string()], // Direct UI -> DB violation
                    exports: vec!["UserCard".to_string()],
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
                    id: "src/db".to_string(),
                    name: "db".to_string(),
                    root_path: "src/db".to_string(),
                    files: vec!["src/db/client.ts".to_string()],
                    imports: vec!["src/services".to_string()], // Cycle with src/services
                    exports: vec!["queryDb".to_string()],
                },
            ],
            dependencies: vec![],
            findings: vec![],
            health: None,
        };

        let engine = RuleEngine::with_recommended_defaults();
        let res = engine.evaluate(&repo);
        assert!(res.is_ok());

        if let Ok(findings) = res {
            let rule_ids: Vec<&str> = findings.iter().map(|f| f.rule_id.as_str()).collect();
            assert_eq!(
                rule_ids,
                vec![
                    "arch/no-circular-dependencies",
                    "arch/no-direct-db-in-ui",
                    "quality/require-service-tests",
                    "quality/max-file-lines",
                ]
            );
        }
    }
}
