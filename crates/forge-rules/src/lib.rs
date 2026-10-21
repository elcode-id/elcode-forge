//! `forge-rules` — Deterministic rule engine and built-in architectural/quality rules.

pub mod builtin;
pub mod declarative;
pub mod engine;
pub mod matchers;
pub mod packs;

pub use builtin::{
    check_max_file_lines, check_no_circular_dependencies, check_no_direct_db_in_ui,
    check_require_service_tests, RULE_MAX_FILE_LINES, RULE_NO_CIRCULAR_DEPS,
    RULE_NO_DIRECT_DB_IN_UI, RULE_REQUIRE_SERVICE_TESTS,
};
pub use declarative::{
    load_declarative_rules, parse_rule_file, validate_rule, validate_rule_id, DeclarativeRule,
    MatcherKind, RuleCategory, RuleFile, RuleMatcher, RuleSource,
};
pub use engine::{RuleConfig, RuleEngine};
pub use forge_core::Analyzer;
pub use matchers::evaluate_declarative_rule;
pub use packs::{get_builtin_pack, list_builtin_packs, resolve_rule_hierarchy, RulePack};
