use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use forge_core::{FileNode, ForgeError, Module};
use serde::{Deserialize, Serialize};

use crate::python_imports::{
    extract_pypi_package_name, extract_python_imports_and_exports, resolve_python_import,
};
use crate::ts_imports::{
    extract_npm_package_name, extract_sfc_script_content, extract_ts_imports_and_exports,
    resolve_import_specifier,
};

/// Inferred architecture graph for repositories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureGraph {
    pub modules: Vec<Module>,
    pub circular_dependencies: Vec<Vec<String>>,
    pub high_centrality_modules: Vec<String>,
    pub external_imports: BTreeSet<String>,
}

#[derive(Default)]
struct ModuleAccumulator {
    name: String,
    root_path: String,
    files: BTreeSet<String>,
    imports: BTreeSet<String>,
    exports: BTreeSet<String>,
}

/// Builds the [`ArchitectureGraph`] for all TypeScript, JavaScript, Vue, and Svelte files in `files`,
/// inferring logical module boundaries, external npm imports, high-centrality modules, and
/// circular dependencies using Tarjan's Strongly Connected Components (SCC) algorithm.
pub fn build_ts_architecture_graph(
    repo_root: &Path,
    files: &[FileNode],
) -> Result<ArchitectureGraph, ForgeError> {
    let ts_js_files: Vec<&FileNode> = files
        .iter()
        .filter(|f| {
            matches!(
                f.language.as_deref(),
                Some("typescript") | Some("javascript") | Some("vue") | Some("svelte")
            )
        })
        .collect();

    let known_files: BTreeSet<String> = ts_js_files
        .iter()
        .map(|f| f.relative_path.clone())
        .collect();

    let mut module_map: BTreeMap<String, ModuleAccumulator> = BTreeMap::new();
    let mut external_imports = BTreeSet::new();

    for file in &ts_js_files {
        let (mod_id, mod_name) = infer_module_boundary(&file.relative_path);
        let acc = module_map
            .entry(mod_id.clone())
            .or_insert_with(|| ModuleAccumulator {
                name: mod_name,
                root_path: mod_id.clone(),
                ..Default::default()
            });
        acc.files.insert(file.relative_path.clone());
    }

    for file in &ts_js_files {
        let (source_mod_id, _) = infer_module_boundary(&file.relative_path);
        let full_path = repo_root.join(&file.relative_path);
        let raw_content = fs::read_to_string(&full_path).map_err(|source| ForgeError::Io {
            path: full_path,
            source,
        })?;

        let content = if matches!(file.language.as_deref(), Some("vue") | Some("svelte")) {
            extract_sfc_script_content(&raw_content)
        } else {
            raw_content
        };

        let (raw_imports, file_exports) = extract_ts_imports_and_exports(&content);

        if let Some(acc) = module_map.get_mut(&source_mod_id) {
            for exp in file_exports {
                acc.exports.insert(exp);
            }
        }

        for spec in raw_imports {
            if let Some(target_file) =
                resolve_import_specifier(&file.relative_path, &spec, &known_files)
            {
                let (target_mod_id, _) = infer_module_boundary(&target_file);
                if target_mod_id != source_mod_id {
                    if let Some(acc) = module_map.get_mut(&source_mod_id) {
                        acc.imports.insert(target_mod_id);
                    }
                }
            } else if let Some(pkg) = extract_npm_package_name(&spec) {
                external_imports.insert(pkg);
            }
        }
    }

    let modules: Vec<Module> = module_map
        .into_iter()
        .map(|(id, acc)| Module {
            id,
            name: acc.name,
            root_path: acc.root_path,
            files: acc.files.into_iter().collect(),
            imports: acc.imports.into_iter().collect(),
            exports: acc.exports.into_iter().collect(),
        })
        .collect();

    let circular_dependencies = detect_circular_dependencies(&modules);
    let high_centrality_modules = compute_high_centrality(&modules);

    Ok(ArchitectureGraph {
        modules,
        circular_dependencies,
        high_centrality_modules,
        external_imports,
    })
}

/// Builds the [`ArchitectureGraph`] for all Python files in `files`,
/// inferring logical module boundaries, external PyPI imports, high-centrality modules, and
/// circular dependencies using Tarjan's Strongly Connected Components (SCC) algorithm.
pub fn build_python_architecture_graph(
    repo_root: &Path,
    files: &[FileNode],
) -> Result<ArchitectureGraph, ForgeError> {
    let py_files: Vec<&FileNode> = files
        .iter()
        .filter(|f| {
            matches!(f.language.as_deref(), Some("python")) || f.relative_path.ends_with(".py")
        })
        .collect();

    let known_files: BTreeSet<String> = py_files.iter().map(|f| f.relative_path.clone()).collect();

    let mut module_map: BTreeMap<String, ModuleAccumulator> = BTreeMap::new();
    let mut external_imports = BTreeSet::new();

    for file in &py_files {
        let (mod_id, mod_name) = infer_python_module_boundary(&file.relative_path);
        let acc = module_map
            .entry(mod_id.clone())
            .or_insert_with(|| ModuleAccumulator {
                name: mod_name,
                root_path: mod_id.clone(),
                ..Default::default()
            });
        acc.files.insert(file.relative_path.clone());
    }

    for file in &py_files {
        let (source_mod_id, _) = infer_python_module_boundary(&file.relative_path);
        let full_path = repo_root.join(&file.relative_path);
        let content = fs::read_to_string(&full_path).map_err(|source| ForgeError::Io {
            path: full_path,
            source,
        })?;

        let (raw_imports, file_exports) = extract_python_imports_and_exports(&content);

        if let Some(acc) = module_map.get_mut(&source_mod_id) {
            for exp in file_exports {
                acc.exports.insert(exp);
            }
        }

        for spec in raw_imports {
            if let Some(target_file) =
                resolve_python_import(&file.relative_path, &spec, &known_files)
            {
                let (target_mod_id, _) = infer_python_module_boundary(&target_file);
                if target_mod_id != source_mod_id {
                    if let Some(acc) = module_map.get_mut(&source_mod_id) {
                        acc.imports.insert(target_mod_id);
                    }
                }
            } else if let Some(pkg) = extract_pypi_package_name(&spec) {
                external_imports.insert(pkg);
            }
        }
    }

    let modules: Vec<Module> = module_map
        .into_iter()
        .map(|(id, acc)| Module {
            id,
            name: acc.name,
            root_path: acc.root_path,
            files: acc.files.into_iter().collect(),
            imports: acc.imports.into_iter().collect(),
            exports: acc.exports.into_iter().collect(),
        })
        .collect();

    let circular_dependencies = detect_circular_dependencies(&modules);
    let high_centrality_modules = compute_high_centrality(&modules);

    Ok(ArchitectureGraph {
        modules,
        circular_dependencies,
        high_centrality_modules,
        external_imports,
    })
}

/// Builds the [`ArchitectureGraph`] for all Go files in `files`,
/// inferring logical package boundaries, external module imports, high-centrality modules, and
/// circular package dependencies using Tarjan's Strongly Connected Components (SCC) algorithm.
pub fn build_go_architecture_graph(
    repo_root: &Path,
    files: &[FileNode],
    module_path: Option<&str>,
) -> Result<ArchitectureGraph, ForgeError> {
    let go_files: Vec<&FileNode> = files
        .iter()
        .filter(|f| matches!(f.language.as_deref(), Some("go")) || f.relative_path.ends_with(".go"))
        .collect();

    let known_files: BTreeSet<String> = go_files.iter().map(|f| f.relative_path.clone()).collect();

    let mut module_map: BTreeMap<String, ModuleAccumulator> = BTreeMap::new();
    let mut external_imports = BTreeSet::new();

    for file in &go_files {
        let (mod_id, mod_name) = infer_go_module_boundary(&file.relative_path);
        let acc = module_map
            .entry(mod_id.clone())
            .or_insert_with(|| ModuleAccumulator {
                name: mod_name,
                root_path: mod_id.clone(),
                ..Default::default()
            });
        acc.files.insert(file.relative_path.clone());
    }

    for file in &go_files {
        let (source_mod_id, _) = infer_go_module_boundary(&file.relative_path);
        let full_path = repo_root.join(&file.relative_path);
        let content = fs::read_to_string(&full_path).map_err(|source| ForgeError::Io {
            path: full_path,
            source,
        })?;

        let raw_imports = crate::go_imports::extract_go_imports(&content);

        for spec in raw_imports {
            if let Some(target_file) = crate::go_imports::resolve_go_import(
                &file.relative_path,
                &spec,
                module_path,
                &known_files,
            ) {
                let (target_mod_id, _) = infer_go_module_boundary(&target_file);
                if target_mod_id != source_mod_id {
                    if let Some(acc) = module_map.get_mut(&source_mod_id) {
                        acc.imports.insert(target_mod_id);
                    }
                }
            } else if let Some(pkg) =
                crate::go_imports::extract_go_external_module(&spec, module_path)
            {
                external_imports.insert(pkg);
            }
        }
    }

    let modules: Vec<Module> = module_map
        .into_iter()
        .map(|(id, acc)| Module {
            id,
            name: acc.name,
            root_path: acc.root_path,
            files: acc.files.into_iter().collect(),
            imports: acc.imports.into_iter().collect(),
            exports: acc.exports.into_iter().collect(),
        })
        .collect();

    let circular_dependencies = detect_circular_dependencies(&modules);
    let high_centrality_modules = compute_high_centrality(&modules);

    Ok(ArchitectureGraph {
        modules,
        circular_dependencies,
        high_centrality_modules,
        external_imports,
    })
}

/// Builds the [`ArchitectureGraph`] for all Rust files in `files`,
/// inferring logical crate/module boundaries, external crate imports, high-centrality modules, and
/// circular dependencies using Tarjan's Strongly Connected Components (SCC) algorithm.
pub fn build_rust_architecture_graph(
    repo_root: &Path,
    files: &[FileNode],
) -> Result<ArchitectureGraph, ForgeError> {
    let rust_files: Vec<&FileNode> = files
        .iter()
        .filter(|f| {
            matches!(f.language.as_deref(), Some("rust")) || f.relative_path.ends_with(".rs")
        })
        .collect();

    let known_files: BTreeSet<String> =
        rust_files.iter().map(|f| f.relative_path.clone()).collect();

    let mut module_map: BTreeMap<String, ModuleAccumulator> = BTreeMap::new();
    let mut external_imports = BTreeSet::new();

    for file in &rust_files {
        let (mod_id, mod_name) = infer_rust_module_boundary(&file.relative_path);
        let acc = module_map
            .entry(mod_id.clone())
            .or_insert_with(|| ModuleAccumulator {
                name: mod_name,
                root_path: mod_id.clone(),
                ..Default::default()
            });
        acc.files.insert(file.relative_path.clone());
    }

    for file in &rust_files {
        let (source_mod_id, _) = infer_rust_module_boundary(&file.relative_path);
        let full_path = repo_root.join(&file.relative_path);
        let content = fs::read_to_string(&full_path).map_err(|source| ForgeError::Io {
            path: full_path,
            source,
        })?;

        let (declared_mods, use_paths) = crate::rust_imports::extract_rust_mods_and_uses(&content);

        // 1. Process declared submodules: `mod <name>;`
        for mod_name in declared_mods {
            let mod_spec = format!("mod::{mod_name}");
            if let Some(target_file) = crate::rust_imports::resolve_rust_import(
                &file.relative_path,
                &mod_spec,
                &known_files,
            ) {
                let (target_mod_id, _) = infer_rust_module_boundary(&target_file);
                if target_mod_id != source_mod_id {
                    if let Some(acc) = module_map.get_mut(&source_mod_id) {
                        acc.imports.insert(target_mod_id);
                    }
                }
            }
        }

        // 2. Process use imports
        for spec in use_paths {
            if let Some(target_file) =
                crate::rust_imports::resolve_rust_import(&file.relative_path, &spec, &known_files)
            {
                let (target_mod_id, _) = infer_rust_module_boundary(&target_file);
                if target_mod_id != source_mod_id {
                    if let Some(acc) = module_map.get_mut(&source_mod_id) {
                        acc.imports.insert(target_mod_id);
                    }
                }
            } else if let Some(pkg) = crate::rust_imports::extract_rust_external_crate(&spec) {
                external_imports.insert(pkg);
            }
        }
    }

    let modules: Vec<Module> = module_map
        .into_iter()
        .map(|(id, acc)| Module {
            id,
            name: acc.name,
            root_path: acc.root_path,
            files: acc.files.into_iter().collect(),
            imports: acc.imports.into_iter().collect(),
            exports: acc.exports.into_iter().collect(),
        })
        .collect();

    let circular_dependencies = detect_circular_dependencies(&modules);
    let high_centrality_modules = compute_high_centrality(&modules);

    Ok(ArchitectureGraph {
        modules,
        circular_dependencies,
        high_centrality_modules,
        external_imports,
    })
}

/// Builds a unified, cross-language [`ArchitectureGraph`] covering all detected source files
/// (TypeScript, JavaScript, Python, Go, and Rust) in the repository.
///
/// Ensures clean module namespace isolation across polyglot subprojects while merging
/// shared modules, unifying external dependencies, and evaluating global circular dependencies
/// via Tarjan's Strongly Connected Components (SCC) algorithm.
pub fn build_polyglot_architecture_graph(
    repo_root: &Path,
    files: &[FileNode],
) -> Result<ArchitectureGraph, ForgeError> {
    let has_ts_js = files.iter().any(|f| {
        matches!(
            f.language.as_deref(),
            Some("typescript") | Some("javascript")
        ) || f.relative_path.ends_with(".ts")
            || f.relative_path.ends_with(".tsx")
            || f.relative_path.ends_with(".js")
            || f.relative_path.ends_with(".jsx")
            || f.relative_path.ends_with(".vue")
            || f.relative_path.ends_with(".svelte")
    });

    let has_py = files.iter().any(|f| {
        matches!(f.language.as_deref(), Some("python")) || f.relative_path.ends_with(".py")
    });

    let has_go = files
        .iter()
        .any(|f| matches!(f.language.as_deref(), Some("go")) || f.relative_path.ends_with(".go"));

    let has_rust = files
        .iter()
        .any(|f| matches!(f.language.as_deref(), Some("rust")) || f.relative_path.ends_with(".rs"));

    let mut all_modules: BTreeMap<String, Module> = BTreeMap::new();
    let mut all_external_imports: BTreeSet<String> = BTreeSet::new();

    if has_ts_js {
        let ts_graph = build_ts_architecture_graph(repo_root, files)?;
        for m in ts_graph.modules {
            all_modules
                .entry(m.id.clone())
                .and_modify(|existing| {
                    let mut file_set: BTreeSet<String> = existing.files.drain(..).collect();
                    file_set.extend(m.files.clone());
                    existing.files = file_set.into_iter().collect();

                    let mut import_set: BTreeSet<String> = existing.imports.drain(..).collect();
                    import_set.extend(m.imports.clone());
                    existing.imports = import_set.into_iter().collect();

                    let mut export_set: BTreeSet<String> = existing.exports.drain(..).collect();
                    export_set.extend(m.exports.clone());
                    existing.exports = export_set.into_iter().collect();
                })
                .or_insert(m);
        }
        all_external_imports.extend(ts_graph.external_imports);
    }

    if has_py {
        let py_graph = build_python_architecture_graph(repo_root, files)?;
        for m in py_graph.modules {
            all_modules
                .entry(m.id.clone())
                .and_modify(|existing| {
                    let mut file_set: BTreeSet<String> = existing.files.drain(..).collect();
                    file_set.extend(m.files.clone());
                    existing.files = file_set.into_iter().collect();

                    let mut import_set: BTreeSet<String> = existing.imports.drain(..).collect();
                    import_set.extend(m.imports.clone());
                    existing.imports = import_set.into_iter().collect();

                    let mut export_set: BTreeSet<String> = existing.exports.drain(..).collect();
                    export_set.extend(m.exports.clone());
                    existing.exports = export_set.into_iter().collect();
                })
                .or_insert(m);
        }
        all_external_imports.extend(py_graph.external_imports);
    }

    if has_go {
        let module_path = if repo_root.join("go.mod").is_file() {
            crate::go_manifest::parse_go_manifest(repo_root)
                .ok()
                .and_then(|m| m.module_path)
        } else {
            files
                .iter()
                .find(|f| f.relative_path.ends_with("go.mod"))
                .and_then(|f| {
                    let full = repo_root.join(&f.relative_path);
                    let parent = full.parent().unwrap_or(repo_root);
                    crate::go_manifest::parse_go_manifest(parent)
                        .ok()
                        .and_then(|m| m.module_path)
                })
        };

        let go_graph = build_go_architecture_graph(repo_root, files, module_path.as_deref())?;
        for m in go_graph.modules {
            all_modules
                .entry(m.id.clone())
                .and_modify(|existing| {
                    let mut file_set: BTreeSet<String> = existing.files.drain(..).collect();
                    file_set.extend(m.files.clone());
                    existing.files = file_set.into_iter().collect();

                    let mut import_set: BTreeSet<String> = existing.imports.drain(..).collect();
                    import_set.extend(m.imports.clone());
                    existing.imports = import_set.into_iter().collect();

                    let mut export_set: BTreeSet<String> = existing.exports.drain(..).collect();
                    export_set.extend(m.exports.clone());
                    existing.exports = export_set.into_iter().collect();
                })
                .or_insert(m);
        }
        all_external_imports.extend(go_graph.external_imports);
    }

    if has_rust {
        let rust_graph = build_rust_architecture_graph(repo_root, files)?;
        for m in rust_graph.modules {
            all_modules
                .entry(m.id.clone())
                .and_modify(|existing| {
                    let mut file_set: BTreeSet<String> = existing.files.drain(..).collect();
                    file_set.extend(m.files.clone());
                    existing.files = file_set.into_iter().collect();

                    let mut import_set: BTreeSet<String> = existing.imports.drain(..).collect();
                    import_set.extend(m.imports.clone());
                    existing.imports = import_set.into_iter().collect();

                    let mut export_set: BTreeSet<String> = existing.exports.drain(..).collect();
                    export_set.extend(m.exports.clone());
                    existing.exports = export_set.into_iter().collect();
                })
                .or_insert(m);
        }
        all_external_imports.extend(rust_graph.external_imports);
    }

    let modules: Vec<Module> = all_modules.into_values().collect();
    let circular_dependencies = detect_circular_dependencies(&modules);
    let high_centrality_modules = compute_high_centrality(&modules);

    Ok(ArchitectureGraph {
        modules,
        circular_dependencies,
        high_centrality_modules,
        external_imports: all_external_imports,
    })
}

/// Infers a logical module `(id, name)` from a normalized relative file path for TS/JS projects.
///
/// Examples:
/// - `"src/components/UserCard.tsx"` -> `("src/components", "components")`
/// - `"src/index.ts"` -> `("src", "src")`
fn infer_module_boundary(relative_path: &str) -> (String, String) {
    let parts: Vec<&str> = relative_path.split('/').collect();
    if let Some(src_idx) = parts.iter().position(|&p| p == "src") {
        if parts.len() >= src_idx + 3 {
            let id = parts[..=src_idx + 1].join("/");
            let name = parts[src_idx + 1].to_string();
            return (id, name);
        } else if parts.len() >= src_idx + 2 {
            let id = parts[..=src_idx].join("/");
            let name = parts[src_idx].to_string();
            return (id, name);
        }
    }
    if parts.len() >= 2 {
        let dir_parts = &parts[..parts.len() - 1];
        let id = dir_parts.join("/");
        let name = dir_parts.last().unwrap_or(&"root").to_string();
        return (id, name);
    }
    ("root".to_string(), "root".to_string())
}

/// Infers a logical module `(id, name)` from a normalized relative Python file path.
///
/// Examples:
/// - `"app/routers/items.py"` -> `("app/routers", "routers")`
/// - `"app/services/user.py"` -> `("app/services", "services")`
/// - `"src/my_app/routers/items.py"` -> `("src/my_app/routers", "routers")`
/// - `"src/services/user.py"` -> `("src/services", "services")`
/// - `"main.py"` -> `("root", "root")`
fn infer_python_module_boundary(relative_path: &str) -> (String, String) {
    let parts: Vec<&str> = relative_path.split('/').collect();
    if let Some(src_idx) = parts.iter().position(|&p| p == "src") {
        if parts.len() >= src_idx + 4 {
            let id = parts[..=src_idx + 2].join("/");
            let name = parts[src_idx + 2].to_string();
            return (id, name);
        } else if parts.len() >= src_idx + 3 {
            let id = parts[..=src_idx + 1].join("/");
            let name = parts[src_idx + 1].to_string();
            return (id, name);
        }
    }
    if let Some(app_idx) = parts.iter().position(|&p| p == "app") {
        if parts.len() >= app_idx + 3 {
            let id = parts[..=app_idx + 1].join("/");
            let name = parts[app_idx + 1].to_string();
            return (id, name);
        } else if parts.len() >= app_idx + 2 {
            let id = parts[..=app_idx].join("/");
            let name = parts[app_idx].to_string();
            return (id, name);
        }
    }
    if parts.len() >= 2 {
        let dir_parts = &parts[..parts.len() - 1];
        let id = dir_parts.join("/");
        let name = dir_parts.last().unwrap_or(&"root").to_string();
        return (id, name);
    }
    ("root".to_string(), "root".to_string())
}

/// Infers a logical package `(id, name)` from a normalized relative Go file path.
///
/// In Go, packages are organized by directory:
/// - `"cmd/server/main.go"` -> `("cmd/server", "server")`
/// - `"internal/services/user.go"` -> `("internal/services", "services")`
/// - `"main.go"` -> `("root", "root")`
fn infer_go_module_boundary(relative_path: &str) -> (String, String) {
    let parts: Vec<&str> = relative_path.split('/').collect();
    if parts.len() >= 2 {
        let dir_parts = &parts[..parts.len() - 1];
        let id = dir_parts.join("/");
        let name = dir_parts.last().unwrap_or(&"root").to_string();
        return (id, name);
    }
    ("root".to_string(), "root".to_string())
}

/// Infers a logical crate or submodule `(id, name)` from a normalized relative Rust file path.
///
/// Examples:
/// - `"crates/forge-cli/src/commands/doctor.rs"` -> `("crates/forge-cli/commands", "commands")`
/// - `"crates/forge-analyzer/src/lib.rs"` -> `("crates/forge-analyzer", "forge-analyzer")`
/// - `"src/handlers/auth.rs"` -> `("src/handlers", "handlers")`
/// - `"src/main.rs"` -> `("src", "src")`
pub fn infer_rust_module_boundary(relative_path: &str) -> (String, String) {
    let parts: Vec<&str> = relative_path.split('/').collect();
    if parts.len() >= 2 && (parts[0] == "crates" || parts[0] == "packages") {
        let crate_name = parts[1];
        (
            format!("{}/{}", parts[0], crate_name),
            crate_name.to_string(),
        )
    } else if parts.len() >= 3 && parts[0] == "src" {
        (format!("src/{}", parts[1]), parts[1].to_string())
    } else if parts.len() >= 2 && parts[0] == "src" {
        ("src".to_string(), "src".to_string())
    } else if parts.len() >= 2 {
        let dir = parts[0];
        (dir.to_string(), dir.to_string())
    } else {
        ("root".to_string(), "root".to_string())
    }
}

fn compute_high_centrality(modules: &[Module]) -> Vec<String> {
    let mut in_degree: BTreeMap<&str, usize> = BTreeMap::new();
    let mut out_degree: BTreeMap<&str, usize> = BTreeMap::new();

    for m in modules {
        in_degree.entry(m.id.as_str()).or_insert(0);
        out_degree.insert(m.id.as_str(), m.imports.len());
    }

    for m in modules {
        for imported in &m.imports {
            if let Some(count) = in_degree.get_mut(imported.as_str()) {
                *count += 1;
            }
        }
    }

    let mut ranked: Vec<(String, usize)> = modules
        .iter()
        .filter_map(|m| {
            let deg = in_degree.get(m.id.as_str()).copied().unwrap_or(0)
                + out_degree.get(m.id.as_str()).copied().unwrap_or(0);
            if deg >= 2 {
                Some((m.id.clone(), deg))
            } else {
                None
            }
        })
        .collect();

    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.into_iter().map(|(id, _)| id).collect()
}

/// Tarjan's Strongly Connected Components (SCC) algorithm for deterministic cycle detection.
fn detect_circular_dependencies(modules: &[Module]) -> Vec<Vec<String>> {
    struct TarjanState<'a> {
        adj: BTreeMap<&'a str, Vec<&'a str>>,
        index: usize,
        indices: BTreeMap<&'a str, usize>,
        lowlink: BTreeMap<&'a str, usize>,
        stack: Vec<&'a str>,
        on_stack: BTreeSet<&'a str>,
        cycles: Vec<Vec<String>>,
    }

    fn strongconnect<'a>(v: &'a str, state: &mut TarjanState<'a>) {
        let current_idx = state.index;
        state.indices.insert(v, current_idx);
        state.lowlink.insert(v, current_idx);
        state.index += 1;
        state.stack.push(v);
        state.on_stack.insert(v);

        let neighbors = state.adj.get(v).cloned().unwrap_or_default();
        for w in neighbors {
            if !state.indices.contains_key(w) {
                strongconnect(w, state);
                let low_v = state.lowlink.get(v).copied().unwrap_or(0);
                let low_w = state.lowlink.get(w).copied().unwrap_or(0);
                state.lowlink.insert(v, low_v.min(low_w));
            } else if state.on_stack.contains(w) {
                let low_v = state.lowlink.get(v).copied().unwrap_or(0);
                let idx_w = state.indices.get(w).copied().unwrap_or(0);
                state.lowlink.insert(v, low_v.min(idx_w));
            }
        }

        let low_v = state.lowlink.get(v).copied().unwrap_or(0);
        let idx_v = state.indices.get(v).copied().unwrap_or(0);
        if low_v == idx_v {
            let mut scc = Vec::new();
            while let Some(w) = state.stack.pop() {
                state.on_stack.remove(w);
                scc.push(w.to_string());
                if w == v {
                    break;
                }
            }
            if scc.len() > 1 {
                scc.sort();
                state.cycles.push(scc);
            }
        }
    }

    let mut adj: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for m in modules {
        adj.insert(
            m.id.as_str(),
            m.imports.iter().map(String::as_str).collect(),
        );
    }

    let mut state = TarjanState {
        adj,
        index: 0,
        indices: BTreeMap::new(),
        lowlink: BTreeMap::new(),
        stack: Vec::new(),
        on_stack: BTreeSet::new(),
        cycles: Vec::new(),
    };

    for m in modules {
        if !state.indices.contains_key(m.id.as_str()) {
            strongconnect(m.id.as_str(), &mut state);
        }
    }

    state.cycles.sort();
    state.cycles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::scan_repository;
    use std::path::PathBuf;

    #[test]
    fn builds_architecture_graph_and_detects_cycles_on_fixture_next_app() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let discovery = scan_repository(&fixture, &[]);
        assert!(discovery.is_ok());

        if let Ok(summary) = discovery {
            let graph = build_ts_architecture_graph(&fixture, &summary.files);
            assert!(graph.is_ok());

            if let Ok(g) = graph {
                let mod_ids: Vec<&str> = g.modules.iter().map(|m| m.id.as_str()).collect();
                assert_eq!(
                    mod_ids,
                    vec!["src/components", "src/db", "src/pages", "src/services"]
                );

                // Verify circular dependency between src/db and src/services
                assert_eq!(
                    g.circular_dependencies,
                    vec![vec!["src/db".to_string(), "src/services".to_string()]]
                );

                // Verify external npm package extraction
                assert!(g.external_imports.contains("react"));

                // Verify high centrality modules include src/db and src/services
                assert!(g.high_centrality_modules.contains(&"src/db".to_string()));
            }
        }
    }
}
