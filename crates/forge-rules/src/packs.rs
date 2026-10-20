//! Pre-packaged rule sets and config-driven inheritance hierarchy.

use std::collections::BTreeMap;
use std::path::Path;

use forge_core::{Confidence, ForgeConfig, ForgeError, Severity};
use serde::{Deserialize, Serialize};

use crate::builtin::{
    RULE_MAX_FILE_LINES, RULE_NO_CIRCULAR_DEPS, RULE_NO_DIRECT_DB_IN_UI, RULE_REQUIRE_SERVICE_TESTS,
};
use crate::declarative::{
    load_declarative_rules, DeclarativeRule, MatcherKind, RuleCategory, RuleMatcher, RuleSource,
};
use crate::engine::RuleConfig;

/// Pre-packaged set of architectural, quality, or security rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulePack {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub builtins: Vec<RuleConfig>,
    pub declarative_rules: Vec<DeclarativeRule>,
}

/// Returns the built-in [`RulePack`] matching `name` (case-insensitive), if found.
#[must_use]
pub fn get_builtin_pack(name: &str) -> Option<RulePack> {
    match name.to_ascii_lowercase().replace('_', "-").as_str() {
        "recommended" | "default" => Some(pack_recommended()),
        "strict" => Some(pack_strict()),
        "security" | "sec" => Some(pack_security()),
        "clean-architecture" | "clean-arch" => Some(pack_clean_architecture()),
        _ => None,
    }
}

/// Returns the complete list of built-in [`RulePack`]s in deterministic order.
#[must_use]
pub fn list_builtin_packs() -> Vec<RulePack> {
    vec![
        pack_recommended(),
        pack_strict(),
        pack_security(),
        pack_clean_architecture(),
    ]
}

/// Resolves the full rule configuration hierarchy from `repo_root` and `config`:
///
/// `Built-in Defaults < Extended Packs < Custom Local Rules < Per-rule Overrides`
pub fn resolve_rule_hierarchy(
    repo_root: &Path,
    config: &ForgeConfig,
) -> Result<(Vec<RuleConfig>, Vec<DeclarativeRule>), ForgeError> {
    let mut builtins_map: BTreeMap<String, RuleConfig> = BTreeMap::new();
    let mut declarative_map: BTreeMap<String, DeclarativeRule> = BTreeMap::new();

    // 1. Resolve packs to extend (defaults to ["recommended"] if unconfigured)
    let packs_to_apply = if config.rule_packs.is_empty() {
        vec!["recommended".to_string()]
    } else {
        config.rule_packs.clone()
    };

    for pack_name in &packs_to_apply {
        if let Some(pack) = get_builtin_pack(pack_name) {
            for b in pack.builtins {
                builtins_map.insert(b.id.clone(), b);
            }
            for d in pack.declarative_rules {
                declarative_map.insert(d.id.clone(), d);
            }
        }
    }

    // 2. Load custom local rules (.eforge/rules/*.yaml and configured custom_rule_paths)
    let local_rules = load_declarative_rules(repo_root, &config.custom_rule_paths)?;
    for rule in local_rules {
        declarative_map.insert(rule.id.clone(), rule);
    }

    // 3. Apply per-rule overrides from config.rules (highest precedence)
    for (rule_id, override_cfg) in &config.rules {
        if let Some(b) = builtins_map.get_mut(rule_id) {
            if let Some(enabled) = override_cfg.enabled {
                b.enabled = enabled;
            }
            if let Some(sev) = override_cfg.severity {
                b.severity = sev;
            }
            if let Some(max_lines) = override_cfg.max_lines {
                b.threshold = Some(max_lines);
            }
        } else if let Some(d) = declarative_map.get_mut(rule_id) {
            if let Some(enabled) = override_cfg.enabled {
                d.enabled = enabled;
            }
            if let Some(sev) = override_cfg.severity {
                d.severity = sev;
            }
            if let Some(max_lines) = override_cfg.max_lines {
                d.matcher.max_lines = Some(max_lines);
            }
        } else {
            // Override declares a rule not yet present: check if it's a known built-in rule
            if is_known_builtin_rule(rule_id) {
                let mut b = default_builtin_rule(rule_id);
                if let Some(enabled) = override_cfg.enabled {
                    b.enabled = enabled;
                }
                if let Some(sev) = override_cfg.severity {
                    b.severity = sev;
                }
                if let Some(max_lines) = override_cfg.max_lines {
                    b.threshold = Some(max_lines);
                }
                builtins_map.insert(rule_id.clone(), b);
            }
        }
    }

    let mut resolved_builtins: Vec<RuleConfig> = builtins_map.into_values().collect();
    let mut resolved_declarative: Vec<DeclarativeRule> = declarative_map.into_values().collect();

    resolved_builtins.sort_by(|a, b| a.id.cmp(&b.id));
    resolved_declarative.sort_by(|a, b| a.id.cmp(&b.id));

    Ok((resolved_builtins, resolved_declarative))
}

fn is_known_builtin_rule(id: &str) -> bool {
    matches!(
        id,
        RULE_NO_DIRECT_DB_IN_UI
            | RULE_NO_CIRCULAR_DEPS
            | RULE_REQUIRE_SERVICE_TESTS
            | RULE_MAX_FILE_LINES
    )
}

fn default_builtin_rule(id: &str) -> RuleConfig {
    match id {
        RULE_NO_DIRECT_DB_IN_UI => RuleConfig {
            id: RULE_NO_DIRECT_DB_IN_UI.to_string(),
            enabled: true,
            severity: Severity::High,
            threshold: None,
        },
        RULE_NO_CIRCULAR_DEPS => RuleConfig {
            id: RULE_NO_CIRCULAR_DEPS.to_string(),
            enabled: true,
            severity: Severity::High,
            threshold: None,
        },
        RULE_REQUIRE_SERVICE_TESTS => RuleConfig {
            id: RULE_REQUIRE_SERVICE_TESTS.to_string(),
            enabled: true,
            severity: Severity::Medium,
            threshold: None,
        },
        RULE_MAX_FILE_LINES => RuleConfig {
            id: RULE_MAX_FILE_LINES.to_string(),
            enabled: true,
            severity: Severity::Low,
            threshold: Some(300),
        },
        _ => RuleConfig {
            id: id.to_string(),
            enabled: true,
            severity: Severity::Low,
            threshold: None,
        },
    }
}

fn pack_recommended() -> RulePack {
    RulePack {
        id: "recommended".to_string(),
        name: "Recommended Defaults".to_string(),
        description: "Standard ELCODE Forge engineering baseline preventing circular dependencies, direct UI-to-DB calls, missing service tests, and long files.".to_string(),
        version: "1.0.0".to_string(),
        builtins: vec![
            default_builtin_rule(RULE_NO_DIRECT_DB_IN_UI),
            default_builtin_rule(RULE_NO_CIRCULAR_DEPS),
            default_builtin_rule(RULE_REQUIRE_SERVICE_TESTS),
            default_builtin_rule(RULE_MAX_FILE_LINES),
        ],
        declarative_rules: Vec::new(),
    }
}

fn pack_strict() -> RulePack {
    RulePack {
        id: "strict".to_string(),
        name: "Strict Quality & Boundaries".to_string(),
        description: "Elevated engineering discipline enforcing critical architecture boundaries, strict 250-line limits, test companions, and unpinned version prevention.".to_string(),
        version: "1.0.0".to_string(),
        builtins: vec![
            RuleConfig {
                id: RULE_NO_DIRECT_DB_IN_UI.to_string(),
                enabled: true,
                severity: Severity::Critical,
                threshold: None,
            },
            RuleConfig {
                id: RULE_NO_CIRCULAR_DEPS.to_string(),
                enabled: true,
                severity: Severity::Critical,
                threshold: None,
            },
            RuleConfig {
                id: RULE_REQUIRE_SERVICE_TESTS.to_string(),
                enabled: true,
                severity: Severity::High,
                threshold: None,
            },
            RuleConfig {
                id: RULE_MAX_FILE_LINES.to_string(),
                enabled: true,
                severity: Severity::Medium,
                threshold: Some(250),
            },
        ],
        declarative_rules: vec![
            DeclarativeRule {
                id: "quality/strict-test-companions".to_string(),
                name: "Strict Test Companions".to_string(),
                category: RuleCategory::Quality,
                severity: Severity::High,
                confidence: Confidence::Certain,
                description: "All services and utility files must have corresponding test companion files.".to_string(),
                matcher: RuleMatcher {
                    kind: MatcherKind::RequiredCompanion,
                    files: vec!["src/services/**".to_string(), "src/utils/**".to_string()],
                    companion: Some("tests/{name}.test.ts".to_string()),
                    ..Default::default()
                },
                remediation: "Create companion test file in tests/ directory.".to_string(),
                source: RuleSource::Pack("strict".to_string()),
                enabled: true,
            },
            DeclarativeRule {
                id: "deps/no-wildcard-versions".to_string(),
                name: "No Wildcard Versions".to_string(),
                category: RuleCategory::Dependencies,
                severity: Severity::High,
                confidence: Confidence::Certain,
                description: "Dependencies must specify bounded or exact versions instead of wildcards.".to_string(),
                matcher: RuleMatcher {
                    kind: MatcherKind::DependencyRestriction,
                    disallowed_packages: vec!["wildcard-version".to_string()],
                    ..Default::default()
                },
                remediation: "Pin dependency to a specific semver range.".to_string(),
                source: RuleSource::Pack("strict".to_string()),
                enabled: true,
            },
        ],
    }
}

fn pack_security() -> RulePack {
    RulePack {
        id: "security".to_string(),
        name: "Security & Unsafe Code Hardening".to_string(),
        description: "Hardens code against code injection (eval), command execution, debugger statements, dangerous inner HTML, and vulnerable dependencies.".to_string(),
        version: "1.0.0".to_string(),
        builtins: vec![
            default_builtin_rule(RULE_NO_DIRECT_DB_IN_UI),
            default_builtin_rule(RULE_NO_CIRCULAR_DEPS),
        ],
        declarative_rules: vec![
            DeclarativeRule {
                id: "security/no-eval".to_string(),
                name: "No Dynamic Code Execution".to_string(),
                category: RuleCategory::Security,
                severity: Severity::Critical,
                confidence: Confidence::Certain,
                description: "Disallow eval() dynamic code evaluation to prevent arbitrary code injection.".to_string(),
                matcher: RuleMatcher {
                    kind: MatcherKind::RegexPattern,
                    files: vec!["**/*.js".to_string(), "**/*.ts".to_string(), "**/*.tsx".to_string(), "**/*.jsx".to_string()],
                    pattern: Some(r"eval\(".to_string()),
                    ..Default::default()
                },
                remediation: "Replace eval() with structured parsing or explicit dispatch.".to_string(),
                source: RuleSource::Pack("security".to_string()),
                enabled: true,
            },
            DeclarativeRule {
                id: "security/no-child-process-exec".to_string(),
                name: "No Child Process Exec".to_string(),
                category: RuleCategory::Security,
                severity: Severity::High,
                confidence: Confidence::Certain,
                description: "Disallow child_process.exec() to prevent arbitrary shell command injection.".to_string(),
                matcher: RuleMatcher {
                    kind: MatcherKind::RegexPattern,
                    files: vec!["**/*.js".to_string(), "**/*.ts".to_string()],
                    pattern: Some(r"child_process(\.exec|/promises\.exec)".to_string()),
                    ..Default::default()
                },
                remediation: "Use execFile or spawn with explicit argument vectors instead of shell string evaluation.".to_string(),
                source: RuleSource::Pack("security".to_string()),
                enabled: true,
            },
            DeclarativeRule {
                id: "security/no-debugger".to_string(),
                name: "No Debugger Statements".to_string(),
                category: RuleCategory::Security,
                severity: Severity::High,
                confidence: Confidence::Certain,
                description: "Disallow debugger statements in production code.".to_string(),
                matcher: RuleMatcher {
                    kind: MatcherKind::RegexPattern,
                    files: vec!["**/*.js".to_string(), "**/*.ts".to_string(), "**/*.tsx".to_string(), "**/*.jsx".to_string()],
                    pattern: Some(r"\bdebugger;?".to_string()),
                    ..Default::default()
                },
                remediation: "Remove debugger statement before shipping.".to_string(),
                source: RuleSource::Pack("security".to_string()),
                enabled: true,
            },
            DeclarativeRule {
                id: "security/no-dangerous-inner-html".to_string(),
                name: "No Dangerous Inner HTML".to_string(),
                category: RuleCategory::Security,
                severity: Severity::High,
                confidence: Confidence::Certain,
                description: "Disallow dangerouslySetInnerHTML to prevent Cross-Site Scripting (XSS).".to_string(),
                matcher: RuleMatcher {
                    kind: MatcherKind::RegexPattern,
                    files: vec!["**/*.tsx".to_string(), "**/*.jsx".to_string(), "**/*.ts".to_string(), "**/*.js".to_string()],
                    pattern: Some(r"dangerouslySetInnerHTML".to_string()),
                    ..Default::default()
                },
                remediation: "Use safe React data bindings or sanitize HTML via DOMPurify.".to_string(),
                source: RuleSource::Pack("security".to_string()),
                enabled: true,
            },
            DeclarativeRule {
                id: "deps/no-insecure-packages".to_string(),
                name: "No Insecure Packages".to_string(),
                category: RuleCategory::Dependencies,
                severity: Severity::High,
                confidence: Confidence::Certain,
                description: "Prohibit known vulnerable or deprecated packages.".to_string(),
                matcher: RuleMatcher {
                    kind: MatcherKind::DependencyRestriction,
                    disallowed_packages: vec![
                        "event-stream".to_string(),
                        "flatmap-stream".to_string(),
                        "request".to_string(),
                    ],
                    ..Default::default()
                },
                remediation: "Replace deprecated or compromised package with an actively maintained alternative.".to_string(),
                source: RuleSource::Pack("security".to_string()),
                enabled: true,
            },
        ],
    }
}

fn pack_clean_architecture() -> RulePack {
    RulePack {
        id: "clean-architecture".to_string(),
        name: "Clean Architecture & Layer Boundaries".to_string(),
        description: "Enforces hexagonal/clean architecture: presentation cannot touch infrastructure or direct DB, and domain core remains purely decoupled.".to_string(),
        version: "1.0.0".to_string(),
        builtins: vec![
            default_builtin_rule(RULE_NO_DIRECT_DB_IN_UI),
            default_builtin_rule(RULE_NO_CIRCULAR_DEPS),
        ],
        declarative_rules: vec![
            DeclarativeRule {
                id: "arch/ui-no-infra".to_string(),
                name: "UI Cannot Import Infrastructure".to_string(),
                category: RuleCategory::Architecture,
                severity: Severity::High,
                confidence: Confidence::Certain,
                description: "Presentation and UI components cannot directly depend on infrastructure modules.".to_string(),
                matcher: RuleMatcher {
                    kind: MatcherKind::ForbiddenImports,
                    files: vec![
                        "src/ui/**".to_string(),
                        "src/components/**".to_string(),
                        "src/pages/**".to_string(),
                    ],
                    forbidden: vec![
                        "src/infra/**".to_string(),
                        "src/infrastructure/**".to_string(),
                    ],
                    ..Default::default()
                },
                remediation: "Access infrastructure capabilities through application services or domain interfaces.".to_string(),
                source: RuleSource::Pack("clean-architecture".to_string()),
                enabled: true,
            },
            DeclarativeRule {
                id: "arch/domain-pure".to_string(),
                name: "Pure Domain Layer".to_string(),
                category: RuleCategory::Architecture,
                severity: Severity::Critical,
                confidence: Confidence::Certain,
                description: "Domain entities and business rules cannot import UI, infrastructure, or controller modules.".to_string(),
                matcher: RuleMatcher {
                    kind: MatcherKind::ForbiddenImports,
                    files: vec![
                        "src/domain/**".to_string(),
                        "src/core/**".to_string(),
                    ],
                    forbidden: vec![
                        "src/ui/**".to_string(),
                        "src/components/**".to_string(),
                        "src/pages/**".to_string(),
                        "src/infra/**".to_string(),
                        "src/infrastructure/**".to_string(),
                        "src/controllers/**".to_string(),
                    ],
                    ..Default::default()
                },
                remediation: "Ensure domain models are pure and do not depend on external layers.".to_string(),
                source: RuleSource::Pack("clean-architecture".to_string()),
                enabled: true,
            },
            DeclarativeRule {
                id: "arch/infra-no-ui".to_string(),
                name: "Infrastructure Cannot Import UI".to_string(),
                category: RuleCategory::Architecture,
                severity: Severity::High,
                confidence: Confidence::Certain,
                description: "Infrastructure adapters must not depend upward on presentation modules.".to_string(),
                matcher: RuleMatcher {
                    kind: MatcherKind::ForbiddenImports,
                    files: vec![
                        "src/infra/**".to_string(),
                        "src/infrastructure/**".to_string(),
                    ],
                    forbidden: vec![
                        "src/ui/**".to_string(),
                        "src/components/**".to_string(),
                        "src/pages/**".to_string(),
                    ],
                    ..Default::default()
                },
                remediation: "Decouple infrastructure from UI presentation code.".to_string(),
                source: RuleSource::Pack("clean-architecture".to_string()),
                enabled: true,
            },
            DeclarativeRule {
                id: "arch/ui-no-db".to_string(),
                name: "UI Cannot Import Database Drivers".to_string(),
                category: RuleCategory::Architecture,
                severity: Severity::High,
                confidence: Confidence::Certain,
                description: "UI presentation code cannot directly import database modules or client libraries.".to_string(),
                matcher: RuleMatcher {
                    kind: MatcherKind::ForbiddenImports,
                    files: vec![
                        "src/ui/**".to_string(),
                        "src/components/**".to_string(),
                        "src/pages/**".to_string(),
                    ],
                    forbidden: vec![
                        "src/db/**".to_string(),
                        "@prisma/client".to_string(),
                        "pg".to_string(),
                        "mysql".to_string(),
                        "mysql2".to_string(),
                        "typeorm".to_string(),
                        "sqlite3".to_string(),
                        "better-sqlite3".to_string(),
                        "mongoose".to_string(),
                    ],
                    ..Default::default()
                },
                remediation: "Route database queries through service layer or data access repositories.".to_string(),
                source: RuleSource::Pack("clean-architecture".to_string()),
                enabled: true,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::RuleOverrideConfig;
    use std::fs;

    #[test]
    fn provides_four_distinct_builtin_packs() {
        let packs = list_builtin_packs();
        assert_eq!(packs.len(), 4);
        let ids: Vec<&str> = packs.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["recommended", "strict", "security", "clean-architecture"]
        );

        assert!(get_builtin_pack("recommended").is_some());
        assert!(get_builtin_pack("strict").is_some());
        assert!(get_builtin_pack("security").is_some());
        assert!(get_builtin_pack("clean-architecture").is_some());
        assert!(get_builtin_pack("clean_architecture").is_some());
        assert!(get_builtin_pack("unknown-pack").is_none());
    }

    #[test]
    fn resolves_default_hierarchy_to_recommended_when_unconfigured() {
        let temp_dir = std::env::temp_dir().join(format!("forge_pack_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let config = ForgeConfig::default();
        let res = resolve_rule_hierarchy(&temp_dir, &config);
        assert!(res.is_ok());

        if let Ok((builtins, declarative)) = res {
            assert_eq!(builtins.len(), 4);
            assert!(declarative.is_empty());

            let builtin_ids: Vec<&str> = builtins.iter().map(|b| b.id.as_str()).collect();
            assert!(builtin_ids.contains(&RULE_NO_DIRECT_DB_IN_UI));
            assert!(builtin_ids.contains(&RULE_NO_CIRCULAR_DEPS));
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn resolves_precedence_extended_packs_overridden_by_rules_config() {
        let temp_dir =
            std::env::temp_dir().join(format!("forge_pack_test_prec_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let mut config = ForgeConfig {
            rule_packs: vec!["strict".to_string(), "security".to_string()],
            ..Default::default()
        };

        // User overrides quality/max-file-lines threshold to 500 and disables security/no-debugger
        config.rules.insert(
            RULE_MAX_FILE_LINES.to_string(),
            RuleOverrideConfig {
                enabled: Some(true),
                severity: Some(Severity::Low),
                max_lines: Some(500),
            },
        );
        config.rules.insert(
            "security/no-debugger".to_string(),
            RuleOverrideConfig {
                enabled: Some(false),
                severity: None,
                max_lines: None,
            },
        );

        let res = resolve_rule_hierarchy(&temp_dir, &config);
        assert!(res.is_ok());

        if let Ok((builtins, declarative)) = res {
            let max_lines_rule = builtins.iter().find(|b| b.id == RULE_MAX_FILE_LINES);
            assert!(max_lines_rule.is_some());
            if let Some(r) = max_lines_rule {
                assert_eq!(r.threshold, Some(500));
                assert_eq!(r.severity, Severity::Low);
            }

            let debugger_rule = declarative.iter().find(|d| d.id == "security/no-debugger");
            assert!(debugger_rule.is_some());
            if let Some(d) = debugger_rule {
                assert!(!d.enabled);
            }
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
