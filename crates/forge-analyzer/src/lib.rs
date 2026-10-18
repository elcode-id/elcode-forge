//! `forge-analyzer` — Project discovery, AST/import graph, and language/framework detectors.

pub mod detector;
pub mod scanner;

pub use detector::{detect_ecosystem, detect_language, is_test_file, EcosystemSignals};
pub use forge_core::Analyzer;
pub use scanner::{scan_repository, DiscoverySummary};
