//! `eforge init` command — initializes `eforge.config.yaml` and `.eforge/` directory,
//! with optional graph-theoretic topology & boundary inference (`--infer`).

use std::fs;
use std::path::{Path, PathBuf};

use forge_analyzer::{
    build_polyglot_architecture_graph, infer_topology_and_boundaries, scanner::scan_repository,
};
use forge_core::{cache::ForgeCache, config::PRIMARY_CONFIG_FILE, ForgeError};

const DEFAULT_CONFIG_TEMPLATE: &str = r#"# ELCODE Forge (eforge) Configuration
version: "1"

project_name: "elcode-forge-project"
languages:
  - "typescript"
  - "javascript"

ignore_patterns:
  - "node_modules/**"
  - "dist/**"
  - "build/**"
  - ".next/**"
  - "coverage/**"
  - ".eforge/**"

git_enabled: true
fail_on_severity: "high"
"#;

/// Generates a starter `eforge.config.yaml` and initializes the `.eforge/` workspace directory.
///
/// If `eforge.config.yaml` already exists, returns its path without overwriting.
pub fn run_init(repo_root: &Path) -> Result<PathBuf, ForgeError> {
    run_init_with_options(repo_root, false, false)
}

/// Initializes `eforge.config.yaml` and `.eforge/` with optional `--force` and `--infer` topology discovery.
pub fn run_init_with_options(
    repo_root: &Path,
    force: bool,
    infer: bool,
) -> Result<PathBuf, ForgeError> {
    let _cache = ForgeCache::init(repo_root)?;
    let config_path = repo_root.join(PRIMARY_CONFIG_FILE);

    if !infer {
        if !config_path.exists() || force {
            fs::write(&config_path, DEFAULT_CONFIG_TEMPLATE).map_err(|source| ForgeError::Io {
                path: config_path.clone(),
                source,
            })?;
        }
        return Ok(config_path);
    }

    // Topology & Boundary Inference mode (`eforge init --infer`)
    let discovery = scan_repository(repo_root, &[])?;
    let graph = build_polyglot_architecture_graph(repo_root, &discovery.files)?;
    let inference = infer_topology_and_boundaries(&discovery.files, &graph);

    let rules_dir = repo_root.join(".eforge").join("rules");
    fs::create_dir_all(&rules_dir).map_err(|source| ForgeError::Io {
        path: rules_dir.clone(),
        source,
    })?;
    let inferred_rules_path = rules_dir.join("inferred-architecture.yaml");
    fs::write(&inferred_rules_path, &inference.generated_rules_yaml).map_err(|source| {
        ForgeError::Io {
            path: inferred_rules_path,
            source,
        }
    })?;

    if !config_path.exists() || force {
        let project_name = repo_root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("elcode-forge-project");
        let langs_yaml = if discovery.languages.is_empty() {
            "  - \"typescript\"\n".to_string()
        } else {
            discovery
                .languages
                .iter()
                .map(|l| format!("  - \"{l}\"\n"))
                .collect::<String>()
        };
        let packs_yaml = inference
            .recommended_packs
            .iter()
            .map(|p| format!("\"{p}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let inferred_config = format!(
            "# ELCODE Forge (eforge) Configuration — Inferred Archetype: {}\nversion: \"1\"\n\nproject_name: \"{}\"\nlanguages:\n{}\nextends: [{}]\ncustom_rule_paths:\n  - \".eforge/rules/inferred-architecture.yaml\"\n\nignore_patterns:\n  - \"node_modules/**\"\n  - \"dist/**\"\n  - \"build/**\"\n  - \".next/**\"\n  - \"target/**\"\n  - \".eforge/**\"\n\ngit_enabled: true\nfail_on_severity: \"high\"\n",
            inference.archetype_label, project_name, langs_yaml, packs_yaml
        );
        fs::write(&config_path, inferred_config).map_err(|source| ForgeError::Io {
            path: config_path.clone(),
            source,
        })?;
    }

    Ok(config_path)
}
