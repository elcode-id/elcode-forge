//! JSON Schema generation for [`ReportBundle`] and related IR data structures.

use std::fs;
use std::path::Path;

use forge_core::ForgeError;
use schemars::schema_for;

use crate::commands::report::ReportBundle;

/// Generates a deterministic, pretty-printed JSON Schema for [`ReportBundle`].
pub fn render_report_schema() -> Result<String, ForgeError> {
    let schema = schema_for!(ReportBundle);
    serde_json::to_string_pretty(&schema).map_err(|err| ForgeError::Serialization {
        message: format!("Failed to serialize ReportBundle JSON Schema: {err}"),
    })
}

/// Executes the `eforge schema` subcommand, optionally writing the generated JSON schema to `output`.
pub fn run_schema(output: Option<&Path>) -> Result<String, ForgeError> {
    let rendered = render_report_schema()?;
    if let Some(out_path) = output {
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|source| ForgeError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
        }
        fs::write(out_path, &rendered).map_err(|source| ForgeError::Io {
            path: out_path.to_path_buf(),
            source,
        })?;
    }
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_report_schema_contains_expected_definitions() -> Result<(), ForgeError> {
        let schema_json = render_report_schema()?;
        assert!(schema_json.contains("\"title\": \"ReportBundle\""));
        assert!(schema_json.contains("\"schema_version\""));
        assert!(schema_json.contains("\"ArchitectureSummary\""));
        assert!(schema_json.contains("\"Finding\""));
        assert!(schema_json.contains("\"HealthScore\""));
        assert!(schema_json.contains("\"GitRevision\""));
        assert!(schema_json.contains("\"HotspotEntry\""));
        assert!(schema_json.contains("\"ChangeImpactReport\""));
        assert!(schema_json.contains("\"RiskReport\""));

        // Validate that schema is valid JSON
        let val: serde_json::Value =
            serde_json::from_str(&schema_json).map_err(|err| ForgeError::Serialization {
                message: format!("Schema is not valid JSON: {err}"),
            })?;
        assert!(val.is_object());
        Ok(())
    }

    #[test]
    fn test_render_report_schema_is_deterministic() -> Result<(), ForgeError> {
        let first = render_report_schema()?;
        let second = render_report_schema()?;
        assert_eq!(first, second);
        Ok(())
    }

    #[test]
    fn test_fixture_bundle_matches_schema_structure() -> Result<(), ForgeError> {
        use crate::commands::report::{build_report_bundle, ReportOptions};
        use std::path::PathBuf;

        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let opts = ReportOptions {
            hotspots_top_n: 10,
            include_git: false,
        };

        let bundle = build_report_bundle(&fixture, &opts)?;
        let bundle_json =
            serde_json::to_string_pretty(&bundle).map_err(|err| ForgeError::Serialization {
                message: format!("Failed to serialize bundle: {err}"),
            })?;

        let schema_str = render_report_schema()?;
        let schema_val: serde_json::Value =
            serde_json::from_str(&schema_str).map_err(|err| ForgeError::Serialization {
                message: format!("Schema JSON parse error: {err}"),
            })?;
        let bundle_val: serde_json::Value =
            serde_json::from_str(&bundle_json).map_err(|err| ForgeError::Serialization {
                message: format!("Bundle JSON parse error: {err}"),
            })?;

        assert!(schema_val.is_object());
        assert!(bundle_val.is_object());
        assert_eq!(bundle_val["schema_version"], 1);
        assert!(bundle_val["repository"]["name"].is_string());
        assert!(bundle_val["architecture"]["circular_dependencies"].is_array());
        Ok(())
    }
}
