//! `eforge dependencies` command — focused package dependency health & license report.

use std::path::Path;

use forge_analyzer::{
    analyze_polyglot_dependencies, build_polyglot_architecture_graph, scanner::scan_repository,
};
use forge_core::{config::load_config, ForgeError};
use forge_security::{
    categorize_license, detect_dependency_license, detect_project_license, SecurityScanner,
};

use crate::cli::OutputFormat;

/// Analyzes package dependencies in `repo_root` and renders the report in `format`.
pub fn run_dependencies(repo_root: &Path, format: OutputFormat) -> Result<String, ForgeError> {
    let config = load_config(repo_root)?;
    let discovery = scan_repository(repo_root, &config.ignore_patterns)?;
    let arch_graph = build_polyglot_architecture_graph(repo_root, &discovery.files)?;
    let mut report =
        analyze_polyglot_dependencies(repo_root, &discovery.files, &arch_graph.external_imports)?;

    // Integrate license compliance findings
    let license_findings = SecurityScanner::scan_license_compliance(
        repo_root,
        config.license.as_ref(),
        &report.dependencies,
    );
    report.findings.extend(license_findings);
    report
        .findings
        .sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));

    let project_lic_info = detect_project_license(repo_root);

    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(&report).map_err(|err| ForgeError::Serialization {
                message: err.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut md = String::new();
            md.push_str("# ELCODE Forge Dependency Health\n\n");
            let project_lic_str = project_lic_info
                .map(|p| format!("`{}` ({})", p.spdx_id, p.source))
                .unwrap_or_else(|| "None / Unspecified".to_string());
            md.push_str(&format!(
                "- **Project License:** {}\n- **Total Declared Dependencies:** {}\n- **Unused Packages:** {}\n- **Unpinned / Wildcard Versions:** {}\n- **Duplicate Declarations:** {}\n\n",
                project_lic_str,
                report.dependencies.len(),
                report.unused_dependencies.len(),
                report.wildcard_or_unpinned.len(),
                report.duplicate_packages.len()
            ));
            if !report.dependencies.is_empty() {
                md.push_str(
                    "| Package | Ecosystem | Version Req | Resolved | Type |\n| :--- | :--- | :--- | :--- | :--- |\n",
                );
                for dep in &report.dependencies {
                    md.push_str(&format!(
                        "| `{}` | `{}` | `{}` | `{}` | {} |\n",
                        dep.name,
                        dep.ecosystem,
                        dep.version_req,
                        dep.resolved_version.as_deref().unwrap_or("-"),
                        if dep.is_dev { "dev" } else { "prod" }
                    ));
                }

                md.push_str("\n### Open-Source License Inventory\n\n");
                md.push_str(
                    "| Package | Ecosystem | License | Category |\n| :--- | :--- | :--- | :--- |\n",
                );
                for dep in &report.dependencies {
                    let lic = detect_dependency_license(repo_root, dep);
                    let cat = categorize_license(&lic);
                    md.push_str(&format!(
                        "| `{}` | `{}` | `{}` | `{}` |\n",
                        dep.name, dep.ecosystem, lic, cat
                    ));
                }
            }
            if !report.findings.is_empty() {
                md.push_str("\n### Findings\n\n");
                for f in &report.findings {
                    md.push_str(&format!(
                        "- [{:?}] **{}** ({})\n",
                        f.severity, f.title, f.rule_id
                    ));
                }
            }
            Ok(md)
        }
        OutputFormat::Terminal => {
            let mut out = String::new();
            out.push_str("ELCODE Forge Dependency Health & License Compliance\n");
            out.push_str("--------------------------------------------------\n");
            let project_lic_str = project_lic_info
                .map(|p| format!("{} ({})", p.spdx_id, p.source))
                .unwrap_or_else(|| "None / Unspecified".to_string());
            out.push_str(&format!(
                "Project License:      {}\nDeclared packages:    {}\nUnused packages:      {}\nWildcard/unpinned:    {}\nDuplicate packages:   {}\n",
                project_lic_str,
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
