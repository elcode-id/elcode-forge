//! `forge-rules` — Deterministic rule engine and built-in architectural/quality rules.

pub mod builtin;
pub mod engine;

pub use builtin::{
    check_max_file_lines, check_no_circular_dependencies, check_no_direct_db_in_ui,
    check_require_service_tests, RULE_MAX_FILE_LINES, RULE_NO_CIRCULAR_DEPS,
    RULE_NO_DIRECT_DB_IN_UI, RULE_REQUIRE_SERVICE_TESTS,
};
pub use engine::{RuleConfig, RuleEngine};
pub use forge_core::Analyzer;
