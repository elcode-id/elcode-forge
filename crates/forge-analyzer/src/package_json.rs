use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use forge_core::{Dependency, ForgeError};
use serde_json::Value;

/// Parsed result of a repository's `package.json` (and optional `package-lock.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedManifest {
    pub dependencies: Vec<Dependency>,
    pub duplicate_packages: Vec<String>,
}

/// Parses `<repo_root>/package.json` (and resolves exact versions from `package-lock.json` when
/// present) into a deterministically ordered list of [`Dependency`] items.
///
/// If `package.json` does not exist in `repo_root`, returns an empty [`ParsedManifest`].
pub fn parse_package_json(repo_root: &Path) -> Result<ParsedManifest, ForgeError> {
    let manifest_path = repo_root.join("package.json");
    if !manifest_path.is_file() {
        return Ok(ParsedManifest {
            dependencies: Vec::new(),
            duplicate_packages: Vec::new(),
        });
    }

    let raw = fs::read_to_string(&manifest_path).map_err(|source| ForgeError::Io {
        path: manifest_path.clone(),
        source,
    })?;

    let json: Value = serde_json::from_str(&raw).map_err(|err| ForgeError::Serialization {
        message: format!("failed to parse '{}': {err}", manifest_path.display()),
    })?;

    let resolved_versions = read_npm_lockfile_versions(repo_root);

    let mut prod_names = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    let mut deps = Vec::new();

    if let Some(prod_obj) = json.get("dependencies").and_then(Value::as_object) {
        for (name, ver_val) in prod_obj {
            let version_req = ver_val.as_str().unwrap_or("").trim().to_string();
            prod_names.insert(name.clone());
            deps.push(Dependency {
                name: name.clone(),
                version_req,
                resolved_version: resolved_versions.get(name).cloned(),
                ecosystem: "npm".to_string(),
                is_dev: false,
                is_direct: true,
            });
        }
    }

    if let Some(dev_obj) = json.get("devDependencies").and_then(Value::as_object) {
        for (name, ver_val) in dev_obj {
            if prod_names.contains(name) {
                duplicates.insert(name.clone());
            }
            let version_req = ver_val.as_str().unwrap_or("").trim().to_string();
            deps.push(Dependency {
                name: name.clone(),
                version_req,
                resolved_version: resolved_versions.get(name).cloned(),
                ecosystem: "npm".to_string(),
                is_dev: true,
                is_direct: true,
            });
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.is_dev.cmp(&b.is_dev)));

    Ok(ParsedManifest {
        dependencies: deps,
        duplicate_packages: duplicates.into_iter().collect(),
    })
}

fn read_npm_lockfile_versions(repo_root: &Path) -> BTreeMap<String, String> {
    let mut versions = BTreeMap::new();
    let lock_path = repo_root.join("package-lock.json");
    if !lock_path.is_file() {
        return versions;
    }

    let Ok(raw) = fs::read_to_string(&lock_path) else {
        return versions;
    };
    let Ok(json) = serde_json::from_str::<Value>(&raw) else {
        return versions;
    };

    if let Some(packages) = json.get("packages").and_then(Value::as_object) {
        for (pkg_key, pkg_val) in packages {
            if let Some(name) = pkg_key.strip_prefix("node_modules/") {
                if let Some(ver) = pkg_val.get("version").and_then(Value::as_str) {
                    versions.insert(name.to_string(), ver.to_string());
                }
            }
        }
    }

    versions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_fixture_next_app_package_json_deterministically() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let parsed = parse_package_json(&fixture);
        assert!(parsed.is_ok());

        if let Ok(m) = parsed {
            let names: Vec<&str> = m.dependencies.iter().map(|d| d.name.as_str()).collect();
            assert_eq!(
                names,
                vec![
                    "legacy-unpinned-pkg",
                    "next",
                    "react",
                    "react-dom",
                    "typescript",
                    "unused-analytics-sdk",
                    "vitest",
                ]
            );
            assert!(m.duplicate_packages.is_empty());
        }
    }

    #[test]
    fn detects_duplicate_packages_across_dependencies_and_dev_dependencies() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = env::temp_dir().join(format!("eforge_pkg_json_dup_{nanos}"));
        assert!(fs::create_dir_all(&dir).is_ok());

        let pkg_json = r#"{
          "name": "dup-test",
          "dependencies": { "zod": "^3.22.0" },
          "devDependencies": { "zod": "^3.23.0" }
        }"#;
        assert!(fs::write(dir.join("package.json"), pkg_json).is_ok());

        let parsed = parse_package_json(&dir);
        assert!(parsed.is_ok());
        if let Ok(m) = parsed {
            assert_eq!(m.duplicate_packages, vec!["zod"]);
        }

        let _ = fs::remove_dir_all(&dir);
    }
}
