//! `eforge init` command — initializes `eforge.config.yaml` and `.eforge/` directory.

use std::fs;
use std::path::{Path, PathBuf};

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
    let _cache = ForgeCache::init(repo_root)?;

    let config_path = repo_root.join(PRIMARY_CONFIG_FILE);
    if !config_path.exists() {
        fs::write(&config_path, DEFAULT_CONFIG_TEMPLATE).map_err(|source| ForgeError::Io {
            path: config_path.clone(),
            source,
        })?;
    }

    Ok(config_path)
}
