use crate::error::ForgeError;
use crate::ir::{Finding, Repository};

/// Core trait implemented by all deterministic domain analyzers and rule evaluators.
pub trait Analyzer: Send + Sync {
    /// Unique static identifier for the analyzer.
    fn name(&self) -> &'static str;

    /// Inspects the normalized [`Repository`] IR and returns deterministic [`Finding`]s.
    fn analyze(&self, repo: &Repository) -> Result<Vec<Finding>, ForgeError>;
}
