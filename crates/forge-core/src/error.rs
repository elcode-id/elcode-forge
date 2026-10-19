use std::path::PathBuf;
use thiserror::Error;

/// Canonical error type across ELCODE Forge crates.
#[derive(Debug, Error)]
pub enum ForgeError {
    /// I/O error with actionable path context.
    #[error("I/O error at '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Path traversal or symlink escape attempt outside the repository boundary.
    #[error(
        "security boundary violation: path '{candidate}' escapes repository root '{repo_root}'"
    )]
    PathTraversal {
        repo_root: PathBuf,
        candidate: PathBuf,
    },

    /// Configuration parsing or validation error.
    #[error("configuration error: {message}")]
    Config { message: String },

    /// Serialization or deserialization error.
    #[error("serialization error: {message}")]
    Serialization { message: String },

    /// Deterministic analysis error inside an analyzer or rule.
    #[error("analysis error in '{analyzer}': {message}")]
    Analysis {
        analyzer: &'static str,
        message: String,
    },

    /// AI provider or execution error.
    #[error("AI error: {message}")]
    Ai { message: String },

    /// Daily AI spending limit exceeded.
    #[error("AI daily budget exceeded: spent ${spent_usd:.4}, limit ${limit_usd:.4}")]
    AiBudgetExceeded { spent_usd: f64, limit_usd: f64 },
}
