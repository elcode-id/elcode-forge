use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use forge_core::{Dependency, ForgeError};
use toml::Value as TomlValue;

/// Parsed results of dependencies: (dependencies, wildcard_dependencies, path_dependencies).
pub type ParsedCargoDeps = (Vec<Dependency>, Vec<String>, Vec<String>);

/// Parsed results of a Rust crate or workspace manifest and lockfile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRustWorkspace {
    pub is_workspace: bool,
    pub workspace_members: Vec<String>,
    pub dependencies: Vec<Dependency>,
    pub wildcard_dependencies: Vec<String>,
    pub path_dependencies: Vec<String>,
    pub has_cargo_lock: bool,
}

/// Reads `<repo_root>/Cargo.lock` and maps crate names to exact resolved versions.
pub fn read_cargo_lock_versions(repo_root: &Path) -> BTreeMap<String, String> {
    let lock_path = repo_root.join("Cargo.lock");
    let mut map = BTreeMap::new();

    let Ok(content) = fs::read_to_string(&lock_path) else {
        return map;
    };

    let Ok(parsed) = toml::from_str::<TomlValue>(&content) else {
        return map;
    };

    if let Some(packages) = parsed.get("package").and_then(TomlValue::as_array) {
        for pkg in packages {
            if let (Some(name), Some(version)) = (
                pkg.get("name").and_then(TomlValue::as_str),
                pkg.get("version").and_then(TomlValue::as_str),
            ) {
                map.insert(name.to_string(), version.to_string());
            }
        }
    }

    map
}

/// Parses dependencies from a TOML table under `[dependencies]`, `[dev-dependencies]`, or `[build-dependencies]`.
pub fn parse_cargo_dependencies(
    table: &TomlValue,
    section: &str,
    is_dev: bool,
    lock_versions: &BTreeMap<String, String>,
) -> ParsedCargoDeps {
    let mut dependencies = Vec::new();
    let mut wildcard_deps = Vec::new();
    let mut path_deps = Vec::new();

    let Some(sec_val) = table.get(section).and_then(TomlValue::as_table) else {
        return (dependencies, wildcard_deps, path_deps);
    };

    for (name, val) in sec_val {
        let (version_req, is_path, is_wildcard, lookup_name) = match val {
            TomlValue::String(ver) => {
                let trimmed = ver.trim();
                let is_wild = trimmed == "*";
                (trimmed.to_string(), false, is_wild, name.as_str())
            }
            TomlValue::Table(dep_table) => {
                let is_workspace = dep_table
                    .get("workspace")
                    .and_then(TomlValue::as_bool)
                    .unwrap_or(false);
                let has_path = dep_table.get("path").is_some();
                let raw_ver = dep_table.get("version").and_then(TomlValue::as_str);
                let pkg_name = dep_table
                    .get("package")
                    .and_then(TomlValue::as_str)
                    .unwrap_or(name);

                let ver_str = if let Some(v) = raw_ver {
                    v.to_string()
                } else if is_workspace {
                    "workspace".to_string()
                } else if has_path {
                    "path".to_string()
                } else {
                    "*".to_string()
                };

                let is_wild = ver_str == "*";
                (ver_str, has_path, is_wild, pkg_name)
            }
            _ => ("*".to_string(), false, true, name.as_str()),
        };

        if is_wildcard {
            wildcard_deps.push(name.clone());
        }
        if is_path {
            path_deps.push(name.clone());
        }

        let resolved_version = lock_versions.get(lookup_name).cloned();

        dependencies.push(Dependency {
            name: name.clone(),
            version_req,
            resolved_version,
            ecosystem: "cargo".to_string(),
            is_dev,
            is_direct: true,
        });
    }

    (dependencies, wildcard_deps, path_deps)
}

/// Parses a `Cargo.toml` file content directly.
pub fn parse_cargo_toml_content(
    content: &str,
    lock_versions: &BTreeMap<String, String>,
) -> Result<ParsedCargoDeps, ForgeError> {
    let parsed: TomlValue =
        toml::from_str(content).map_err(|source| ForgeError::Serialization {
            message: format!("Failed to parse Cargo.toml: {source}"),
        })?;

    let mut all_deps = Vec::new();
    let mut all_wildcards = Vec::new();
    let mut all_paths = Vec::new();

    for (section, is_dev) in [
        ("dependencies", false),
        ("dev-dependencies", true),
        ("build-dependencies", true),
    ] {
        let (deps, wilds, paths) =
            parse_cargo_dependencies(&parsed, section, is_dev, lock_versions);
        all_deps.extend(deps);
        all_wildcards.extend(wilds);
        all_paths.extend(paths);
    }

    if let Some(workspace_table) = parsed.get("workspace").and_then(TomlValue::as_table) {
        let (deps, wilds, paths) = parse_cargo_dependencies(
            &TomlValue::Table(workspace_table.clone()),
            "dependencies",
            false,
            lock_versions,
        );
        all_deps.extend(deps);
        all_wildcards.extend(wilds);
        all_paths.extend(paths);
    }

    Ok((all_deps, all_wildcards, all_paths))
}

/// Parses the full Rust workspace or single crate at `repo_root`.
pub fn parse_rust_workspace(repo_root: &Path) -> Result<ParsedRustWorkspace, ForgeError> {
    let cargo_toml_path = repo_root.join("Cargo.toml");
    if !cargo_toml_path.is_file() {
        return Ok(ParsedRustWorkspace {
            is_workspace: false,
            workspace_members: Vec::new(),
            dependencies: Vec::new(),
            wildcard_dependencies: Vec::new(),
            path_dependencies: Vec::new(),
            has_cargo_lock: false,
        });
    }

    let has_cargo_lock = repo_root.join("Cargo.lock").is_file();
    let lock_versions = read_cargo_lock_versions(repo_root);

    let content = fs::read_to_string(&cargo_toml_path).map_err(|source| ForgeError::Io {
        path: cargo_toml_path.clone(),
        source,
    })?;

    let parsed_toml: TomlValue =
        toml::from_str(&content).map_err(|source| ForgeError::Serialization {
            message: format!("Failed to parse {}: {source}", cargo_toml_path.display()),
        })?;

    let is_workspace = parsed_toml.get("workspace").is_some();
    let mut workspace_members = Vec::new();
    let mut dependencies_map: BTreeMap<(String, bool), Dependency> = BTreeMap::new();
    let mut wildcard_set = BTreeSet::new();
    let mut path_set = BTreeSet::new();

    // 1. Parse root Cargo.toml
    let (root_deps, root_wildcards, root_paths) =
        parse_cargo_toml_content(&content, &lock_versions)?;
    for dep in root_deps {
        dependencies_map.insert((dep.name.clone(), dep.is_dev), dep);
    }
    wildcard_set.extend(root_wildcards);
    path_set.extend(root_paths);

    // 2. If workspace, resolve member directories
    if let Some(workspace_table) = parsed_toml.get("workspace").and_then(TomlValue::as_table) {
        if let Some(members_arr) = workspace_table.get("members").and_then(TomlValue::as_array) {
            for item in members_arr {
                if let Some(pattern) = item.as_str() {
                    let pattern_clean = pattern.trim();
                    if pattern_clean.ends_with("/*") {
                        let base_dir = repo_root.join(pattern_clean.trim_end_matches("/*"));
                        if let Ok(entries) = fs::read_dir(&base_dir) {
                            for entry in entries.flatten() {
                                let sub_dir = entry.path();
                                if sub_dir.join("Cargo.toml").is_file() {
                                    if let Ok(rel) = sub_dir.strip_prefix(repo_root) {
                                        let member_rel = rel.to_string_lossy().replace('\\', "/");
                                        workspace_members.push(member_rel.clone());

                                        let sub_toml_path = sub_dir.join("Cargo.toml");
                                        if let Ok(sub_content) = fs::read_to_string(&sub_toml_path)
                                        {
                                            if let Ok((sub_deps, sub_wilds, sub_paths)) =
                                                parse_cargo_toml_content(
                                                    &sub_content,
                                                    &lock_versions,
                                                )
                                            {
                                                for dep in sub_deps {
                                                    let key = (dep.name.clone(), dep.is_dev);
                                                    if let Some(existing) =
                                                        dependencies_map.get_mut(&key)
                                                    {
                                                        if existing.version_req == "workspace"
                                                            && dep.version_req != "workspace"
                                                        {
                                                            existing.version_req = dep.version_req;
                                                        }
                                                        if existing.resolved_version.is_none()
                                                            && dep.resolved_version.is_some()
                                                        {
                                                            existing.resolved_version =
                                                                dep.resolved_version;
                                                        }
                                                    } else {
                                                        dependencies_map.insert(key, dep);
                                                    }
                                                }
                                                wildcard_set.extend(sub_wilds);
                                                path_set.extend(sub_paths);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        let member_dir = repo_root.join(pattern_clean);
                        if member_dir.join("Cargo.toml").is_file() {
                            workspace_members.push(pattern_clean.to_string());
                            let sub_toml_path = member_dir.join("Cargo.toml");
                            if let Ok(sub_content) = fs::read_to_string(&sub_toml_path) {
                                if let Ok((sub_deps, sub_wilds, sub_paths)) =
                                    parse_cargo_toml_content(&sub_content, &lock_versions)
                                {
                                    for dep in sub_deps {
                                        let key = (dep.name.clone(), dep.is_dev);
                                        if let Some(existing) = dependencies_map.get_mut(&key) {
                                            if existing.version_req == "workspace"
                                                && dep.version_req != "workspace"
                                            {
                                                existing.version_req = dep.version_req;
                                            }
                                            if existing.resolved_version.is_none()
                                                && dep.resolved_version.is_some()
                                            {
                                                existing.resolved_version = dep.resolved_version;
                                            }
                                        } else {
                                            dependencies_map.insert(key, dep);
                                        }
                                    }
                                    wildcard_set.extend(sub_wilds);
                                    path_set.extend(sub_paths);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    workspace_members.sort();
    let mut dependencies: Vec<Dependency> = dependencies_map.into_values().collect();
    dependencies.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.is_dev.cmp(&b.is_dev)));

    Ok(ParsedRustWorkspace {
        is_workspace,
        workspace_members,
        dependencies,
        wildcard_dependencies: wildcard_set.into_iter().collect(),
        path_dependencies: path_set.into_iter().collect(),
        has_cargo_lock,
    })
}
