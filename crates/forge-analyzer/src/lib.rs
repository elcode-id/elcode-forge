//! `forge-analyzer` — Project discovery, AST/import graph, and language/framework detectors.

pub mod dep_health;
pub mod detector;
pub mod package_json;
pub mod scanner;

pub use dep_health::{analyze_npm_dependencies, DependencyReport};
pub use detector::{detect_ecosystem, detect_language, is_test_file, EcosystemSignals};
pub use forge_core::Analyzer;
pub use package_json::{parse_package_json, ParsedManifest};
pub use scanner::{scan_repository, DiscoverySummary};
