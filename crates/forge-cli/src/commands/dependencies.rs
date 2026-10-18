//! `eforge dependencies` command — focused npm/package dependency health report.

use std::path::Path;

use forge_analyzer::{
    dep_health::analyze_npm_dependencies, graph::build_ts_architecture_graph,
    scanner::scan_repository,
};
use forge_core::{config::load_config, ForgeError};

use crate::cli::OutputFormat;

/// Analyzes package dependencies in `repo_root` and renders the report in `format`.
pub fn run_dependencies(repo_root: &Path, format: OutputFormat) -> Result<String, ForgeError> {
    let config = load_config(repo_root)?;
    let discovery = scan_repository(repo_root, &config.ignore_patterns)?;
    let arch_graph = build_ts_architecture_graph(repo_root, &discovery.files)?;
    let report = analyze_npm_dependencies(repo_root, &arch_graph.external_imports)?;

    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(&report).map_err(|err| ForgeError::Serialization {
                message: err.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut md = String::new();
            md.push_str("# ELCODE Forge Dependency Health\n\n");
            md.push_str(&format!(
                "- **Total Declared Dependencies:** {}\n- **Unused Packages:** {}\n- **Unpinned / Wildcard Versions:** {}\n- **Duplicate Declarations:** {}\n\n",
                report.dependencies.len(),
                report.unused_dependencies.len(),
                report.wildcard_or_unpinned.len(),
                report.duplicate_packages.len()
            ));
            if !report.dependencies.is_empty() {
                md.push_str(
                    "| Package | Version Req | Resolved | Type |\n| :--- | :--- | :--- | :--- |\n",
                );
                for dep in &report.dependencies {
                    md.push_str(&format!(
                        "| `{}` | `{}` | `{}` | {} |\n",
                        dep.name,
                        dep.version_req,
                        dep.resolved_version.as_deref().unwrap_or("-"),
                        if dep.is_dev { "dev" } else { "prod" }
                    ));
                }
            }
            Ok(md)
        }
        OutputFormat::Terminal => {
            let mut out = String::new();
            out.push_str("ELCODE Forge Dependency Health\n");
            out.push_str("------------------------------\n");
            out.push_str(&format!(
                "Declared packages:    {}\nUnused packages:      {}\nWildcard/unpinned:    {}\nDuplicate packages:   {}\n",
                report.dependencies.len(),
                report.unused_dependencies.len(),
                report.wildcard_or_unpinned.len(),
                report.duplicate_packages.len()
            ));
            for finding in &report.findings {
                out.push_str(&format!(
                    "  [{:?}] {} ({})\n",
                    finding.severity, finding.title, finding.rule_id
                ));
            }
            Ok(out)
        }
    }
}
