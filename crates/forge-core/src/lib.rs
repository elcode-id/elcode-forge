//! `forge-core` — Foundational Intermediate Representation (IR), error model, and core traits
//! for ELCODE Forge (`eforge`).
//!
//! This crate is the leaf of the workspace dependency DAG and MUST NOT depend on any other
//! internal `forge-*` crate.

pub mod error;
pub mod ir;
pub mod traits;

pub use error::ForgeError;
pub use ir::{
    Confidence, Dependency, DimensionScore, FileNode, Finding, HealthScore, Module, Repository,
    Severity, SourceLocation,
};
pub use traits::Analyzer;
