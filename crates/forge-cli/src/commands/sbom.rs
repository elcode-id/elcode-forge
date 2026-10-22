//! `eforge sbom` command — Software Bill of Materials (SBOM) generator.
//! Exports polyglot dependencies in CycloneDX v1.5 or SPDX v2.3 JSON formats.

use std::fs;
use std::path::Path;

use forge_analyzer::{
    analyze_polyglot_dependencies, build_polyglot_architecture_graph, scanner::scan_repository,
};
use forge_core::{config::load_config, ForgeError};
use forge_security::{generate_sbom, SbomFormat};
use serde_json::Value;

/// Generates an SBOM in the specified `format` for `repo_root`.
pub fn run_sbom(repo_root: &Path, format: SbomFormat) -> Result<String, ForgeError> {
    let config = load_config(repo_root)?;
    let discovery = scan_repository(repo_root, &config.ignore_patterns)?;
    let arch_graph = build_polyglot_architecture_graph(repo_root, &discovery.files)?;
    let dep_report =
        analyze_polyglot_dependencies(repo_root, &discovery.files, &arch_graph.external_imports)?;

    let (project_name, project_version) = detect_project_identity(repo_root);

    generate_sbom(
        format,
        &project_name,
        project_version.as_deref(),
        &dep_report.dependencies,
    )
}

/// Attempts to infer project name and version from standard project manifests.
fn detect_project_identity(repo_root: &Path) -> (String, Option<String>) {
    // 1. Try package.json
    let pkg_json = repo_root.join("package.json");
    if pkg_json.is_file() {
        if let Ok(content) = fs::read_to_string(&pkg_json) {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                let name = val.get("name").and_then(Value::as_str);
                let version = val.get("version").and_then(Value::as_str);
                if let Some(n) = name {
                    return (n.to_string(), version.map(|v| v.to_string()));
                }
            }
        }
    }

    // 2. Try Cargo.toml
    let cargo_toml = repo_root.join("Cargo.toml");
    if cargo_toml.is_file() {
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            if let Ok(val) = toml::from_str::<toml::Value>(&content) {
                let pkg = val
                    .get("package")
                    .or_else(|| val.get("workspace").and_then(|w| w.get("package")));
                if let Some(p) = pkg {
                    let name = p.get("name").and_then(toml::Value::as_str);
                    let version = p.get("version").and_then(toml::Value::as_str);
                    if let Some(n) = name {
                        return (n.to_string(), version.map(|v| v.to_string()));
                    }
                }
            }
        }
    }

    // 3. Try composer.json
    let comp_json = repo_root.join("composer.json");
    if comp_json.is_file() {
        if let Ok(content) = fs::read_to_string(&comp_json) {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                let name = val.get("name").and_then(Value::as_str);
                let version = val.get("version").and_then(Value::as_str);
                if let Some(n) = name {
                    return (n.to_string(), version.map(|v| v.to_string()));
                }
            }
        }
    }

    // 4. Fallback to folder name
    let fallback_name = repo_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();

    (fallback_name, None)
}
