use std::collections::BTreeMap;
use std::path::Path;

use forge_core::{
    Analyzer, Finding, ForgeConfig, ForgeError, Repository, RuleOverrideConfig, Severity,
};
use serde::{Deserialize, Serialize};

use crate::builtin::{
    check_max_file_lines, check_no_circular_dependencies, check_no_direct_db_in_ui,
    check_require_service_tests, RULE_MAX_FILE_LINES, RULE_NO_CIRCULAR_DEPS,
    RULE_NO_DIRECT_DB_IN_UI, RULE_REQUIRE_SERVICE_TESTS,
};
use crate::declarative::DeclarativeRule;
use crate::matchers::evaluate_declarative_rule;

/// Declarative configuration for an individual rule inside [`RuleEngine`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleConfig {
    pub id: String,
    pub enabled: bool,
    pub severity: Severity,
    pub threshold: Option<u32>,
}

/// Deterministic rule evaluation engine.
#[derive(Debug, Clone, Default)]
pub struct RuleEngine {
    rules: Vec<RuleConfig>,
    declarative_rules: Vec<DeclarativeRule>,
}

impl RuleEngine {
    /// Creates a [`RuleEngine`] with the provided custom rule configurations.
    pub fn new(rules: Vec<RuleConfig>) -> Self {
        Self {
            rules,
            declarative_rules: Vec::new(),
        }
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
            declarative_rules: Vec::new(),
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

    /// Builds a [`RuleEngine`] by resolving the full rule configuration hierarchy from `repo_root` and `config`:
    /// `Built-in Defaults < Extended Packs < Custom Local Rules < Per-rule Overrides`.
    pub fn from_config(repo_root: &Path, config: &ForgeConfig) -> Result<Self, ForgeError> {
        let (rules, declarative_rules) = crate::packs::resolve_rule_hierarchy(repo_root, config)?;
        Ok(Self {
            rules,
            declarative_rules,
        })
    }

    /// Appends declarative custom rules into this engine instance.
    #[must_use]
    pub fn with_declarative_rules(mut self, declarative_rules: Vec<DeclarativeRule>) -> Self {
        self.declarative_rules = declarative_rules;
        self
    }

    /// Registers an individual declarative custom rule.
    pub fn add_declarative_rule(&mut self, rule: DeclarativeRule) {
        self.declarative_rules.push(rule);
    }

    /// Returns the slice of configured built-in rules.
    pub fn rules(&self) -> &[RuleConfig] {
        &self.rules
    }

    /// Returns the slice of configured declarative rules.
    pub fn declarative_rules(&self) -> &[DeclarativeRule] {
        &self.declarative_rules
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

        for dec_rule in &self.declarative_rules {
            if !dec_rule.enabled {
                continue;
            }
            findings.extend(evaluate_declarative_rule(dec_rule, repo)?);
        }

        findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
        findings.dedup_by(|a, b| a.id == b.id);
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
    use crate::declarative::{MatcherKind, RuleCategory, RuleMatcher, RuleSource};
    use forge_core::{Confidence, Dependency, FileNode, Module};

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

    #[test]
    fn evaluates_declarative_rules_together_with_builtins() {
        let repo = Repository {
            root_path: "/workspace/app".to_string(),
            name: "app".to_string(),
            detected_languages: vec!["typescript".to_string()],
            detected_frameworks: Vec::new(),
            package_managers: vec!["pnpm".to_string()],
            files: vec![FileNode {
                relative_path: "src/controllers/user_controller.ts".to_string(),
                language: Some("typescript".to_string()),
                size_bytes: 100,
                line_count: 50,
                content_hash: "c1".to_string(),
                is_test: false,
            }],
            modules: Vec::new(),
            dependencies: vec![Dependency {
                name: "moment".to_string(),
                version_req: "^2.29.0".to_string(),
                resolved_version: Some("2.29.4".to_string()),
                ecosystem: "npm".to_string(),
                is_dev: false,
                is_direct: true,
            }],
            findings: Vec::new(),
            health: None,
        };

        let custom_naming_rule = DeclarativeRule {
            id: "naming/controller-suffix".to_string(),
            name: "Controller Suffix".to_string(),
            category: RuleCategory::Quality,
            severity: Severity::Medium,
            confidence: Confidence::Certain,
            description: "Controllers must end with Controller.ts".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::FileNaming,
                files: vec!["src/controllers/**".to_string()],
                convention: Some("*Controller.ts".to_string()),
                ..Default::default()
            },
            remediation: "Rename file".to_string(),
            source: RuleSource::Custom("rules/naming.yaml".to_string()),
            enabled: true,
        };

        let custom_dep_rule = DeclarativeRule {
            id: "deps/no-moment".to_string(),
            name: "No Moment".to_string(),
            category: RuleCategory::Dependencies,
            severity: Severity::Critical,
            confidence: Confidence::Certain,
            description: "Moment is deprecated".to_string(),
            matcher: RuleMatcher {
                kind: MatcherKind::DependencyRestriction,
                disallowed_packages: vec!["moment".to_string()],
                ..Default::default()
            },
            remediation: "Use date-fns".to_string(),
            source: RuleSource::Pack("elcode/quality-pack".to_string()),
            enabled: true,
        };

        let engine = RuleEngine::with_recommended_defaults()
            .with_declarative_rules(vec![custom_naming_rule, custom_dep_rule]);

        let res = engine.evaluate(&repo);
        assert!(res.is_ok());

        if let Ok(findings) = res {
            assert_eq!(findings.len(), 2);
            // Critical dependency finding must come before Medium naming finding
            assert_eq!(findings[0].severity, Severity::Critical);
            assert_eq!(findings[0].rule_id, "deps/no-moment");
            assert_eq!(findings[1].severity, Severity::Medium);
            assert_eq!(findings[1].rule_id, "naming/controller-suffix");
        }
    }

    #[test]
    fn engine_from_config_resolves_extended_packs_and_overrides() {
        let temp_dir =
            std::env::temp_dir().join(format!("forge_engine_cfg_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let mut config = ForgeConfig {
            rule_packs: vec!["clean-architecture".to_string()],
            ..Default::default()
        };
        config.rules.insert(
            "arch/ui-no-infra".to_string(),
            forge_core::RuleOverrideConfig {
                enabled: Some(false),
                severity: None,
                max_lines: None,
            },
        );

        let engine_res = RuleEngine::from_config(&temp_dir, &config);
        assert!(engine_res.is_ok());

        if let Ok(engine) = engine_res {
            let ui_infra = engine
                .declarative_rules()
                .iter()
                .find(|d| d.id == "arch/ui-no-infra");
            assert!(ui_infra.is_some());
            if let Some(d) = ui_infra {
                assert!(!d.enabled);
            }
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
