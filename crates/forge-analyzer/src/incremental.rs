//! Per-file BLAKE3 AST summary cache and `<50ms` incremental architecture graph engine.
//!
//! Enables real-time architectural radar (`eforge watch`), rich call-chain cycle tracing,
//! and surgical file-level import inspection without re-parsing unchanged files.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use forge_core::{CallChainHop, FileImportEdge, FileNode, ForgeError, Module, EFORGE_DIR};
use serde::{Deserialize, Serialize};

use crate::graph::{
    compute_high_centrality, detect_circular_dependencies, infer_rust_module_boundary,
    ArchitectureGraph,
};
use crate::python_imports::{
    extract_pypi_package_name, extract_python_imports_and_exports, resolve_python_import,
};
use crate::ts_imports::{
    extract_npm_package_name, extract_sfc_script_content, extract_ts_imports_and_exports,
    resolve_import_specifier,
};

/// Default filename inside `.eforge/cache/` storing per-file AST import/export summaries.
pub const FILE_AST_CACHE_FILENAME: &str = "file_ast_index.json";

/// Line and column provenance for a raw import specifier within a single file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawImportSpan {
    pub specifier: String,
    pub line: u32,
    pub column: u32,
}

/// Cached AST import/export summary for a single repository file keyed by its BLAKE3 hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedFileAst {
    pub relative_path: String,
    pub content_hash: String,
    pub language: Option<String>,
    pub line_count: u32,
    pub raw_imports: Vec<RawImportSpan>,
    pub exports: Vec<String>,
}

/// Persistent per-file AST summary index stored in `.eforge/cache/file_ast_index.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileAstCache {
    pub entries: BTreeMap<String, CachedFileAst>,
}

impl FileAstCache {
    /// Returns the canonical path to `.eforge/cache/file_ast_index.json` for `repo_root`.
    #[must_use]
    pub fn cache_path(repo_root: &Path) -> PathBuf {
        repo_root
            .join(EFORGE_DIR)
            .join("cache")
            .join(FILE_AST_CACHE_FILENAME)
    }

    /// Loads the per-file AST cache from `.eforge/cache/file_ast_index.json` if present,
    /// returning an empty cache on missing or corrupted files.
    #[must_use]
    pub fn load(repo_root: &Path) -> Self {
        let path = Self::cache_path(repo_root);
        let Ok(raw) = fs::read_to_string(&path) else {
            return Self::default();
        };
        serde_json::from_str::<Self>(&raw).unwrap_or_default()
    }

    /// Saves the per-file AST cache deterministically to `.eforge/cache/file_ast_index.json`.
    pub fn save(&self, repo_root: &Path) -> Result<PathBuf, ForgeError> {
        let path = Self::cache_path(repo_root);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| ForgeError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let serialized =
            serde_json::to_string_pretty(self).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })?;
        fs::write(&path, serialized).map_err(|source| ForgeError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }
}

/// Result of an incremental architecture graph evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncrementalAnalysisResult {
    pub graph: ArchitectureGraph,
    pub file_edges: Vec<FileImportEdge>,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub recomputed_files: Vec<String>,
}

/// Evaluates the polyglot [`ArchitectureGraph`] and file-level [`FileImportEdge`] list using
/// the per-file BLAKE3 [`FileAstCache`]. Only files whose `content_hash` changed (or are absent
/// from the cache) are read and parsed from disk.
pub fn analyze_architecture_incremental(
    repo_root: &Path,
    files: &[FileNode],
    ast_cache: &mut FileAstCache,
) -> Result<IncrementalAnalysisResult, ForgeError> {
    let current_paths: BTreeSet<&str> = files.iter().map(|f| f.relative_path.as_str()).collect();
    ast_cache
        .entries
        .retain(|path, _| current_paths.contains(path.as_str()));

    let mut cache_hits = 0usize;
    let mut cache_misses = 0usize;
    let mut recomputed_files = Vec::new();

    for file in files {
        if !is_supported_source_file(file) {
            continue;
        }

        let is_hit = ast_cache
            .entries
            .get(&file.relative_path)
            .is_some_and(|cached| cached.content_hash == file.content_hash);

        if is_hit {
            cache_hits += 1;
        } else {
            cache_misses += 1;
            recomputed_files.push(file.relative_path.clone());
            let parsed = parse_single_file_ast(repo_root, file)?;
            ast_cache.entries.insert(file.relative_path.clone(), parsed);
        }
    }

    let (graph, file_edges) = assemble_graph_from_cache(repo_root, files, ast_cache);

    Ok(IncrementalAnalysisResult {
        graph,
        file_edges,
        cache_hits,
        cache_misses,
        recomputed_files,
    })
}

/// Extracts all file-level import edges with 1-based line and column numbers across the repository.
pub fn extract_file_import_edges(
    repo_root: &Path,
    files: &[FileNode],
) -> Result<Vec<FileImportEdge>, ForgeError> {
    let mut cache = FileAstCache::load(repo_root);
    let result = analyze_architecture_incremental(repo_root, files, &mut cache)?;
    Ok(result.file_edges)
}

/// Traces each module-level Tarjan SCC cycle into a concrete sequence of file-level [`CallChainHop`]s,
/// marking the hop that acts as the minimum feedback arc candidate (`is_feedback_arc_candidate = true`).
#[must_use]
pub fn trace_cycle_call_chains(
    cycles: &[Vec<String>],
    file_edges: &[FileImportEdge],
) -> Vec<Vec<CallChainHop>> {
    let mut traces = Vec::new();

    for scc in cycles {
        if scc.len() < 2 {
            continue;
        }
        let scc_set: BTreeSet<&str> = scc.iter().map(String::as_str).collect();

        // Collect all inter-module file edges inside this SCC
        let mut scc_edges: Vec<&FileImportEdge> = file_edges
            .iter()
            .filter(|e| {
                e.from_module != e.to_module
                    && scc_set.contains(e.from_module.as_str())
                    && scc_set.contains(e.to_module.as_str())
            })
            .collect();

        scc_edges.sort_by(|a, b| {
            a.from_module
                .cmp(&b.from_module)
                .then_with(|| a.to_module.cmp(&b.to_module))
                .then_with(|| a.from_file.cmp(&b.from_file))
                .then_with(|| a.line.cmp(&b.line))
        });

        // Order hops along the cycle ring where possible
        let mut hops: Vec<CallChainHop> = Vec::new();
        let mut used_pairs: BTreeSet<(&str, &str)> = BTreeSet::new();

        for i in 0..scc.len() {
            let from_m = scc[i].as_str();
            let to_m = scc[(i + 1) % scc.len()].as_str();
            if let Some(edge) = scc_edges
                .iter()
                .find(|e| e.from_module == from_m && e.to_module == to_m)
            {
                used_pairs.insert((edge.from_module.as_str(), edge.to_module.as_str()));
                hops.push(CallChainHop {
                    from_file: edge.from_file.clone(),
                    to_file: edge.to_file.clone(),
                    from_module: edge.from_module.clone(),
                    to_module: edge.to_module.clone(),
                    line: edge.line,
                    column: edge.column,
                    raw_specifier: edge.raw_specifier.clone(),
                    is_feedback_arc_candidate: false,
                });
            }
        }

        // If the SCC wasn't a simple ring in sorted order, include remaining distinct module-pair edges
        if hops.len() < scc.len() {
            for edge in &scc_edges {
                let pair = (edge.from_module.as_str(), edge.to_module.as_str());
                if !used_pairs.contains(&pair) {
                    used_pairs.insert(pair);
                    hops.push(CallChainHop {
                        from_file: edge.from_file.clone(),
                        to_file: edge.to_file.clone(),
                        from_module: edge.from_module.clone(),
                        to_module: edge.to_module.clone(),
                        line: edge.line,
                        column: edge.column,
                        raw_specifier: edge.raw_specifier.clone(),
                        is_feedback_arc_candidate: false,
                    });
                }
            }
        }

        // Mark the closing hop as the primary feedback arc break candidate
        if let Some(last) = hops.last_mut() {
            last.is_feedback_arc_candidate = true;
        }

        if !hops.is_empty() {
            traces.push(hops);
        }
    }

    traces
}

fn is_supported_source_file(file: &FileNode) -> bool {
    matches!(
        file.language.as_deref(),
        Some("typescript" | "javascript" | "vue" | "svelte" | "python" | "go" | "rust")
    ) || file.relative_path.ends_with(".ts")
        || file.relative_path.ends_with(".tsx")
        || file.relative_path.ends_with(".js")
        || file.relative_path.ends_with(".jsx")
        || file.relative_path.ends_with(".vue")
        || file.relative_path.ends_with(".svelte")
        || file.relative_path.ends_with(".py")
        || file.relative_path.ends_with(".go")
        || file.relative_path.ends_with(".rs")
}

fn parse_single_file_ast(repo_root: &Path, file: &FileNode) -> Result<CachedFileAst, ForgeError> {
    let full_path = repo_root.join(&file.relative_path);
    let raw_content = fs::read_to_string(&full_path).map_err(|source| ForgeError::Io {
        path: full_path,
        source,
    })?;

    let lang = detect_file_lang(file);
    let mut raw_imports = Vec::new();
    let mut exports = Vec::new();

    match lang {
        "typescript" | "javascript" | "vue" | "svelte" => {
            let content = if lang == "vue" || lang == "svelte" {
                extract_sfc_script_content(&raw_content)
            } else {
                raw_content.clone()
            };
            let (specs, exps) = extract_ts_imports_and_exports(&content);
            exports = exps;
            for spec in specs {
                let (line, column) = find_specifier_line_col(&raw_content, &spec);
                raw_imports.push(RawImportSpan {
                    specifier: spec,
                    line,
                    column,
                });
            }
        }
        "python" => {
            let (specs, exps) = extract_python_imports_and_exports(&raw_content);
            exports = exps;
            for spec in specs {
                let (line, column) = find_specifier_line_col(&raw_content, &spec);
                raw_imports.push(RawImportSpan {
                    specifier: spec,
                    line,
                    column,
                });
            }
        }
        "go" => {
            let specs = crate::go_imports::extract_go_imports(&raw_content);
            for spec in specs {
                let (line, column) = find_specifier_line_col(&raw_content, &spec);
                raw_imports.push(RawImportSpan {
                    specifier: spec,
                    line,
                    column,
                });
            }
        }
        "rust" => {
            let (declared_mods, use_paths) =
                crate::rust_imports::extract_rust_mods_and_uses(&raw_content);
            for mod_name in declared_mods {
                let spec = format!("mod::{mod_name}");
                let (line, column) = find_specifier_line_col(&raw_content, &mod_name);
                raw_imports.push(RawImportSpan {
                    specifier: spec,
                    line,
                    column,
                });
            }
            for spec in use_paths {
                let search_token = spec.split("::").next().unwrap_or(&spec);
                let (line, column) = find_specifier_line_col(&raw_content, search_token);
                raw_imports.push(RawImportSpan {
                    specifier: spec,
                    line,
                    column,
                });
            }
        }
        _ => {}
    }

    Ok(CachedFileAst {
        relative_path: file.relative_path.clone(),
        content_hash: file.content_hash.clone(),
        language: file.language.clone(),
        line_count: file.line_count,
        raw_imports,
        exports,
    })
}

fn detect_file_lang(file: &FileNode) -> &str {
    if let Some(lang) = file.language.as_deref() {
        return lang;
    }
    if file.relative_path.ends_with(".ts") || file.relative_path.ends_with(".tsx") {
        "typescript"
    } else if file.relative_path.ends_with(".js") || file.relative_path.ends_with(".jsx") {
        "javascript"
    } else if file.relative_path.ends_with(".vue") {
        "vue"
    } else if file.relative_path.ends_with(".svelte") {
        "svelte"
    } else if file.relative_path.ends_with(".py") {
        "python"
    } else if file.relative_path.ends_with(".go") {
        "go"
    } else if file.relative_path.ends_with(".rs") {
        "rust"
    } else {
        "unknown"
    }
}

fn find_specifier_line_col(source: &str, needle: &str) -> (u32, u32) {
    if needle.is_empty() {
        return (1, 1);
    }
    for (idx, line) in source.lines().enumerate() {
        if let Some(byte_pos) = line.find(needle) {
            let col = line[..byte_pos].chars().count() as u32 + 1;
            return (idx as u32 + 1, col);
        }
    }
    (1, 1)
}

#[derive(Default)]
struct IncrementalModuleAcc {
    name: String,
    root_path: String,
    files: BTreeSet<String>,
    imports: BTreeSet<String>,
    exports: BTreeSet<String>,
}

fn assemble_graph_from_cache(
    repo_root: &Path,
    files: &[FileNode],
    ast_cache: &FileAstCache,
) -> (ArchitectureGraph, Vec<FileImportEdge>) {
    let mut ts_files = BTreeSet::new();
    let mut py_files = BTreeSet::new();
    let mut go_files = BTreeSet::new();
    let mut rs_files = BTreeSet::new();

    for file in files {
        match detect_file_lang(file) {
            "typescript" | "javascript" | "vue" | "svelte" => {
                ts_files.insert(file.relative_path.clone());
            }
            "python" => {
                py_files.insert(file.relative_path.clone());
            }
            "go" => {
                go_files.insert(file.relative_path.clone());
            }
            "rust" => {
                rs_files.insert(file.relative_path.clone());
            }
            _ => {}
        }
    }

    let go_module_path = if !go_files.is_empty() && repo_root.join("go.mod").is_file() {
        crate::go_manifest::parse_go_manifest(repo_root)
            .ok()
            .and_then(|m| m.module_path)
    } else {
        None
    };

    let mut module_map: BTreeMap<String, IncrementalModuleAcc> = BTreeMap::new();
    let mut external_imports: BTreeSet<String> = BTreeSet::new();
    let mut file_edges: BTreeSet<FileImportEdge> = BTreeSet::new();

    for file in files {
        let lang = detect_file_lang(file);
        if lang == "unknown" {
            continue;
        }
        let (mod_id, mod_name) = infer_boundary_for_lang(&file.relative_path, lang);
        let acc = module_map
            .entry(mod_id.clone())
            .or_insert_with(|| IncrementalModuleAcc {
                name: mod_name,
                root_path: mod_id.clone(),
                ..Default::default()
            });
        acc.files.insert(file.relative_path.clone());

        let Some(cached) = ast_cache.entries.get(&file.relative_path) else {
            continue;
        };

        for exp in &cached.exports {
            acc.exports.insert(exp.clone());
        }

        for imp in &cached.raw_imports {
            let resolved_target = match lang {
                "typescript" | "javascript" | "vue" | "svelte" => {
                    let target =
                        resolve_import_specifier(&file.relative_path, &imp.specifier, &ts_files);
                    if target.is_none() {
                        if let Some(pkg) = extract_npm_package_name(&imp.specifier) {
                            external_imports.insert(pkg);
                        }
                    }
                    target
                }
                "python" => {
                    let target =
                        resolve_python_import(&file.relative_path, &imp.specifier, &py_files);
                    if target.is_none() {
                        if let Some(pkg) = extract_pypi_package_name(&imp.specifier) {
                            external_imports.insert(pkg);
                        }
                    }
                    target
                }
                "go" => {
                    let target = crate::go_imports::resolve_go_import(
                        &file.relative_path,
                        &imp.specifier,
                        go_module_path.as_deref(),
                        &go_files,
                    );
                    if target.is_none() {
                        if let Some(pkg) = crate::go_imports::extract_go_external_module(
                            &imp.specifier,
                            go_module_path.as_deref(),
                        ) {
                            external_imports.insert(pkg);
                        }
                    }
                    target
                }
                "rust" => {
                    let target = crate::rust_imports::resolve_rust_import(
                        &file.relative_path,
                        &imp.specifier,
                        &rs_files,
                    );
                    if target.is_none() && !imp.specifier.starts_with("mod::") {
                        if let Some(pkg) =
                            crate::rust_imports::extract_rust_external_crate(&imp.specifier)
                        {
                            external_imports.insert(pkg);
                        }
                    }
                    target
                }
                _ => None,
            };

            if let Some(target_file) = resolved_target {
                let (target_mod_id, _) = infer_boundary_for_lang(&target_file, lang);
                if target_mod_id != mod_id {
                    if let Some(source_acc) = module_map.get_mut(&mod_id) {
                        source_acc.imports.insert(target_mod_id.clone());
                    }
                }
                file_edges.insert(FileImportEdge {
                    from_file: file.relative_path.clone(),
                    to_file: target_file,
                    from_module: mod_id.clone(),
                    to_module: target_mod_id,
                    raw_specifier: imp.specifier.clone(),
                    line: imp.line,
                    column: imp.column,
                });
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

    (
        ArchitectureGraph {
            modules,
            circular_dependencies,
            high_centrality_modules,
            external_imports,
        },
        file_edges.into_iter().collect(),
    )
}

pub(crate) fn infer_boundary_for_lang(relative_path: &str, lang: &str) -> (String, String) {
    match lang {
        "rust" => infer_rust_module_boundary(relative_path),
        "go" => {
            let parts: Vec<&str> = relative_path.split('/').collect();
            if parts.len() >= 2 {
                let dir_parts = &parts[..parts.len() - 1];
                let id = dir_parts.join("/");
                let name = dir_parts.last().unwrap_or(&"root").to_string();
                (id, name)
            } else {
                ("root".to_string(), "root".to_string())
            }
        }
        "python" => {
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
                (id, name)
            } else {
                ("root".to_string(), "root".to_string())
            }
        }
        _ => {
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
                (id, name)
            } else {
                ("root".to_string(), "root".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::scan_repository;

    #[test]
    fn incremental_analysis_caches_unchanged_files_and_traces_cycles() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let discovery = scan_repository(&fixture, &[]);
        assert!(discovery.is_ok());
        let Ok(summary) = discovery else { return };

        let mut cache = FileAstCache::default();
        let cold = analyze_architecture_incremental(&fixture, &summary.files, &mut cache);
        assert!(cold.is_ok());
        let Ok(cold_res) = cold else { return };

        assert_eq!(cold_res.cache_hits, 0);
        assert!(cold_res.cache_misses > 0);
        assert!(!cold_res.file_edges.is_empty());

        // Second pass with identical hashes must achieve 100% cache hit rate
        let warm = analyze_architecture_incremental(&fixture, &summary.files, &mut cache);
        assert!(warm.is_ok());
        let Ok(warm_res) = warm else { return };

        assert_eq!(warm_res.cache_misses, 0);
        assert_eq!(warm_res.cache_hits, cold_res.cache_misses);
        assert_eq!(warm_res.graph, cold_res.graph);

        let traces =
            trace_cycle_call_chains(&warm_res.graph.circular_dependencies, &warm_res.file_edges);
        assert_eq!(traces.len(), 1);
        assert!(traces[0].len() >= 2);
        assert!(traces[0].iter().any(|h| h.is_feedback_arc_candidate));
    }
}
