//! `forge-core` — Foundational Intermediate Representation (IR), error model, configuration
//! loader, canonical path guard, and core traits for ELCODE Forge (`eforge`).
//!
//! This crate is the leaf of the workspace dependency DAG and MUST NOT depend on any other
//! internal `forge-*` crate.

pub mod config;
pub mod error;
pub mod ir;
pub mod path_guard;
pub mod traits;

pub use config::{
    load_config, parse_config_yaml, ForgeConfig, FALLBACK_CONFIG_FILE, PRIMARY_CONFIG_FILE,
};
pub use error::ForgeError;
pub use ir::{
    Confidence, Dependency, DimensionScore, FileNode, Finding, HealthScore, Module, Repository,
    Severity, SourceLocation,
};
pub use path_guard::resolve_safe_path;
pub use traits::Analyzer;
