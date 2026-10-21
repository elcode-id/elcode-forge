use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use forge_core::{Dependency, ForgeError};
use serde_json::Value as JsonValue;
use toml::Value as TomlValue;

/// Parsed results of a repository's Python dependencies across `pyproject.toml`, `requirements.txt`,
/// and lockfiles (`poetry.lock`, `uv.lock`, `Pipfile.lock`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPythonManifest {
    pub dependencies: Vec<Dependency>,
    pub duplicate_packages: Vec<String>,
    pub unpinned_packages: Vec<String>,
}

/// Normalizes a Python package name according to PEP 503 (lowercased, underscores to hyphens).
pub fn normalize_python_package_name(raw: &str) -> String {
    let stripped = match raw.find('[') {
        Some(idx) => &raw[..idx],
        None => raw,
    };
    stripped.trim().to_ascii_lowercase().replace('_', "-")
}

/// Checks whether a declared Python version requirement is unpinned or wildcard.
pub fn is_python_unpinned(version_req: &str) -> bool {
    let trimmed = version_req.trim();
    if trimmed.is_empty()
        || trimmed == "*"
        || trimmed == ">=0"
        || trimmed == ">=0.0"
        || trimmed == ">=0.0.0"
    {
        return true;
    }
    // Only less-than bounds without lower bound, e.g. "<2.0"
    if trimmed.starts_with('<') && !trimmed.contains('>') && !trimmed.contains("==") {
        return true;
    }
    false
}

/// Parses a Python requirement specifier line (e.g. `fastapi>=0.100.0`, `requests[security] == 2.31.0; os_name != 'nt'`).
///
/// Returns `Some((normalized_name, version_req))` or `None` if the line is empty/comment/directive.
pub fn parse_requirement_line(line: &str) -> Option<(String, String)> {
    let mut clean = line.trim();

    // Strip inline comments
    if let Some(comment_idx) = clean.find('#') {
        clean = clean[..comment_idx].trim();
    }

    if clean.is_empty() {
        return None;
    }

    // Skip flags and directives like `-r`, `-i`, `-f`, `--extra-index-url`, `-e`
    if clean.starts_with('-') {
        return None;
    }

    // Strip environment markers after ';'
    if let Some(marker_idx) = clean.find(';') {
        clean = clean[..marker_idx].trim();
    }

    if clean.is_empty() {
        return None;
    }

    // Version specifier delimiters in order of search
    let op_indices: Vec<usize> = clean
        .match_indices(['=', '>', '<', '~', '!', '@'])
        .map(|(idx, _)| idx)
        .collect();

    if let Some(&first_op_idx) = op_indices.first() {
        let raw_name = clean[..first_op_idx].trim();
        let version_spec = clean[first_op_idx..].trim();
        let normalized = normalize_python_package_name(raw_name);
        if normalized.is_empty() {
            None
        } else {
            Some((normalized, version_spec.to_string()))
        }
    } else {
        // No version constraint specified (e.g. `fastapi` or `requests[security]`)
        let normalized = normalize_python_package_name(clean);
        if normalized.is_empty() {
            None
        } else {
            Some((normalized, String::new()))
        }
    }
}

/// Parses `<repo_root>/pyproject.toml` and/or `<repo_root>/requirements.txt` into a deterministically
/// ordered list of [`Dependency`] items with PyPI ecosystem tags.
pub fn parse_python_manifests(repo_root: &Path) -> Result<ParsedPythonManifest, ForgeError> {
    let resolved_versions = read_python_lockfile_versions(repo_root);

    let pyproject_path = repo_root.join("pyproject.toml");
    let requirements_path = repo_root.join("requirements.txt");

    let mut all_deps: BTreeMap<(String, bool), Dependency> = BTreeMap::new();
    let mut prod_names = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    let mut unpinned = BTreeSet::new();

    // 1. Parse pyproject.toml if present
    if pyproject_path.is_file() {
        let raw = fs::read_to_string(&pyproject_path).map_err(|source| ForgeError::Io {
            path: pyproject_path.clone(),
            source,
        })?;
        let manifest = parse_pyproject_toml_content(&raw, &resolved_versions)?;
        for dep in manifest.dependencies {
            if !dep.is_dev {
                prod_names.insert(dep.name.clone());
            } else if prod_names.contains(&dep.name) {
                duplicates.insert(dep.name.clone());
            }
            if is_python_unpinned(&dep.version_req) {
                unpinned.insert(dep.name.clone());
            }
            all_deps.insert((dep.name.clone(), dep.is_dev), dep);
        }
        for dup in manifest.duplicate_packages {
            duplicates.insert(dup);
        }
        for unp in manifest.unpinned_packages {
            unpinned.insert(unp);
        }
    }

    // 2. Parse requirements.txt if present
    if requirements_path.is_file() {
        let raw = fs::read_to_string(&requirements_path).map_err(|source| ForgeError::Io {
            path: requirements_path.clone(),
            source,
        })?;
        let manifest = parse_requirements_txt_content(&raw, &resolved_versions)?;
        for dep in manifest.dependencies {
            if prod_names.contains(&dep.name) {
                duplicates.insert(dep.name.clone());
            }
            prod_names.insert(dep.name.clone());
            if is_python_unpinned(&dep.version_req) {
                unpinned.insert(dep.name.clone());
            }
            all_deps.insert((dep.name.clone(), dep.is_dev), dep);
        }
        for dup in manifest.duplicate_packages {
            duplicates.insert(dup);
        }
        for unp in manifest.unpinned_packages {
            unpinned.insert(unp);
        }
    }

    let mut dependencies: Vec<Dependency> = all_deps.into_values().collect();
    dependencies.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.is_dev.cmp(&b.is_dev)));

    Ok(ParsedPythonManifest {
        dependencies,
        duplicate_packages: duplicates.into_iter().collect(),
        unpinned_packages: unpinned.into_iter().collect(),
    })
}

/// Parses the raw string contents of a `pyproject.toml` file (supporting both PEP 621 and Poetry).
pub fn parse_pyproject_toml_content(
    content: &str,
    lock_versions: &BTreeMap<String, String>,
) -> Result<ParsedPythonManifest, ForgeError> {
    let toml_val: TomlValue = toml::from_str(content).map_err(|err| ForgeError::Serialization {
        message: format!("failed to parse pyproject.toml: {err}"),
    })?;

    let mut deps = Vec::new();
    let mut prod_names = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    let mut unpinned = BTreeSet::new();

    // A. PEP 621 standard: [project.dependencies]
    if let Some(project) = toml_val.get("project").and_then(TomlValue::as_table) {
        if let Some(prod_list) = project.get("dependencies").and_then(TomlValue::as_array) {
            for item in prod_list {
                if let Some(spec) = item.as_str() {
                    if let Some((name, version_req)) = parse_requirement_line(spec) {
                        if prod_names.contains(&name) {
                            duplicates.insert(name.clone());
                        }
                        prod_names.insert(name.clone());
                        if is_python_unpinned(&version_req) {
                            unpinned.insert(name.clone());
                        }
                        deps.push(Dependency {
                            name: name.clone(),
                            version_req,
                            resolved_version: lock_versions.get(&name).cloned(),
                            ecosystem: "pypi".to_string(),
                            is_dev: false,
                            is_direct: true,
                        });
                    }
                }
            }
        }

        // PEP 621: [project.optional-dependencies]
        if let Some(opt_table) = project
            .get("optional-dependencies")
            .and_then(TomlValue::as_table)
        {
            for (_group, list_val) in opt_table {
                if let Some(list) = list_val.as_array() {
                    for item in list {
                        if let Some(spec) = item.as_str() {
                            if let Some((name, version_req)) = parse_requirement_line(spec) {
                                if prod_names.contains(&name) {
                                    duplicates.insert(name.clone());
                                }
                                if is_python_unpinned(&version_req) {
                                    unpinned.insert(name.clone());
                                }
                                deps.push(Dependency {
                                    name: name.clone(),
                                    version_req,
                                    resolved_version: lock_versions.get(&name).cloned(),
                                    ecosystem: "pypi".to_string(),
                                    is_dev: true,
                                    is_direct: true,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // B. Poetry: [tool.poetry.dependencies]
    if let Some(tool) = toml_val.get("tool").and_then(TomlValue::as_table) {
        if let Some(poetry) = tool.get("poetry").and_then(TomlValue::as_table) {
            if let Some(prod_deps) = poetry.get("dependencies").and_then(TomlValue::as_table) {
                for (raw_name, val) in prod_deps {
                    let name = normalize_python_package_name(raw_name);
                    // Skip python itself
                    if name == "python" {
                        continue;
                    }
                    let version_req = extract_poetry_version(val);
                    if prod_names.contains(&name) {
                        duplicates.insert(name.clone());
                    }
                    prod_names.insert(name.clone());
                    if is_python_unpinned(&version_req) {
                        unpinned.insert(name.clone());
                    }
                    deps.push(Dependency {
                        name: name.clone(),
                        version_req,
                        resolved_version: lock_versions.get(&name).cloned(),
                        ecosystem: "pypi".to_string(),
                        is_dev: false,
                        is_direct: true,
                    });
                }
            }

            // Poetry groups: [tool.poetry.group.<group>.dependencies]
            if let Some(group_table) = poetry.get("group").and_then(TomlValue::as_table) {
                for (_group_name, group_val) in group_table {
                    if let Some(g_deps) =
                        group_val.get("dependencies").and_then(TomlValue::as_table)
                    {
                        for (raw_name, val) in g_deps {
                            let name = normalize_python_package_name(raw_name);
                            if name == "python" {
                                continue;
                            }
                            let version_req = extract_poetry_version(val);
                            if prod_names.contains(&name) {
                                duplicates.insert(name.clone());
                            }
                            if is_python_unpinned(&version_req) {
                                unpinned.insert(name.clone());
                            }
                            deps.push(Dependency {
                                name: name.clone(),
                                version_req,
                                resolved_version: lock_versions.get(&name).cloned(),
                                ecosystem: "pypi".to_string(),
                                is_dev: true,
                                is_direct: true,
                            });
                        }
                    }
                }
            }

            // Legacy Poetry: [tool.poetry.dev-dependencies]
            if let Some(dev_deps) = poetry.get("dev-dependencies").and_then(TomlValue::as_table) {
                for (raw_name, val) in dev_deps {
                    let name = normalize_python_package_name(raw_name);
                    if name == "python" {
                        continue;
                    }
                    let version_req = extract_poetry_version(val);
                    if prod_names.contains(&name) {
                        duplicates.insert(name.clone());
                    }
                    if is_python_unpinned(&version_req) {
                        unpinned.insert(name.clone());
                    }
                    deps.push(Dependency {
                        name: name.clone(),
                        version_req,
                        resolved_version: lock_versions.get(&name).cloned(),
                        ecosystem: "pypi".to_string(),
                        is_dev: true,
                        is_direct: true,
                    });
                }
            }
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.is_dev.cmp(&b.is_dev)));

    Ok(ParsedPythonManifest {
        dependencies: deps,
        duplicate_packages: duplicates.into_iter().collect(),
        unpinned_packages: unpinned.into_iter().collect(),
    })
}

/// Parses the contents of a `requirements.txt` file.
pub fn parse_requirements_txt_content(
    content: &str,
    lock_versions: &BTreeMap<String, String>,
) -> Result<ParsedPythonManifest, ForgeError> {
    let mut deps = Vec::new();
    let mut seen_names = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    let mut unpinned = BTreeSet::new();

    for line in content.lines() {
        if let Some((name, version_req)) = parse_requirement_line(line) {
            if seen_names.contains(&name) {
                duplicates.insert(name.clone());
            }
            seen_names.insert(name.clone());
            if is_python_unpinned(&version_req) {
                unpinned.insert(name.clone());
            }
            deps.push(Dependency {
                name: name.clone(),
                version_req,
                resolved_version: lock_versions.get(&name).cloned(),
                ecosystem: "pypi".to_string(),
                is_dev: false,
                is_direct: true,
            });
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(ParsedPythonManifest {
        dependencies: deps,
        duplicate_packages: duplicates.into_iter().collect(),
        unpinned_packages: unpinned.into_iter().collect(),
    })
}

fn extract_poetry_version(val: &TomlValue) -> String {
    if let Some(s) = val.as_str() {
        s.trim().to_string()
    } else if let Some(tbl) = val.as_table() {
        if let Some(v) = tbl.get("version").and_then(TomlValue::as_str) {
            v.trim().to_string()
        } else if tbl.contains_key("path") {
            "path".to_string()
        } else if tbl.contains_key("git") {
            "git".to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    }
}

/// Extracts exact resolved package versions from `poetry.lock`, `uv.lock`, or `Pipfile.lock` at `repo_root`.
pub fn read_python_lockfile_versions(repo_root: &Path) -> BTreeMap<String, String> {
    let mut versions = BTreeMap::new();

    // 1. uv.lock or poetry.lock (TOML format with [[package]] entries)
    for lock_filename in ["uv.lock", "poetry.lock"] {
        let lock_path = repo_root.join(lock_filename);
        if lock_path.is_file() {
            if let Ok(raw) = fs::read_to_string(&lock_path) {
                if let Ok(toml_val) = toml::from_str::<TomlValue>(&raw) {
                    if let Some(pkgs) = toml_val.get("package").and_then(TomlValue::as_array) {
                        for pkg in pkgs {
                            if let (Some(name), Some(ver)) = (
                                pkg.get("name").and_then(TomlValue::as_str),
                                pkg.get("version").and_then(TomlValue::as_str),
                            ) {
                                versions.insert(
                                    normalize_python_package_name(name),
                                    ver.trim().to_string(),
                                );
                            }
                        }
                    }
                }
            }
            if !versions.is_empty() {
                return versions;
            }
        }
    }

    // 2. Pipfile.lock (JSON format)
    let pipfile_lock = repo_root.join("Pipfile.lock");
    if pipfile_lock.is_file() {
        if let Ok(raw) = fs::read_to_string(&pipfile_lock) {
            if let Ok(json) = serde_json::from_str::<JsonValue>(&raw) {
                for section in ["default", "develop"] {
                    if let Some(section_obj) = json.get(section).and_then(JsonValue::as_object) {
                        for (raw_name, pkg_val) in section_obj {
                            if let Some(ver) = pkg_val.get("version").and_then(JsonValue::as_str) {
                                let clean_ver = ver.trim_start_matches("==").trim().to_string();
                                versions.insert(normalize_python_package_name(raw_name), clean_ver);
                            }
                        }
                    }
                }
            }
        }
    }

    versions
}
