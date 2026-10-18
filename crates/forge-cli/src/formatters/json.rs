//! Deterministic pretty-printed JSON formatter for [`Repository`] reports.

use forge_core::{ForgeError, Repository};

/// Renders a [`Repository`] IR as pretty-printed JSON.
pub fn render_json(repo: &Repository) -> Result<String, ForgeError> {
    serde_json::to_string_pretty(repo).map_err(|err| ForgeError::Serialization {
        message: err.to_string(),
    })
}
