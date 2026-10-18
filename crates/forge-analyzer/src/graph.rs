use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use forge_core::{FileNode, ForgeError, Module};
use serde::{Deserialize, Serialize};

use crate::ts_imports::{
    extract_npm_package_name, extract_ts_imports_and_exports, resolve_import_specifier,
};

/// Inferred architecture graph for TypeScript/JavaScript repositories.
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

/// Builds the [`ArchitectureGraph`] for all TypeScript and JavaScript files in `files`,
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
                Some("typescript") | Some("javascript")
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
        let content = fs::read_to_string(&full_path).map_err(|source| ForgeError::Io {
            path: full_path,
            source,
        })?;

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

/// Infers a logical module `(id, name)` from a normalized relative file path.
///
/// Examples:
/// - `"src/components/UserCard.tsx"` -> `("src/components", "components")`
/// - `"src/index.ts"` -> `("src", "src")`
fn infer_module_boundary(relative_path: &str) -> (String, String) {
    let parts: Vec<&str> = relative_path.split('/').collect();
    if parts.len() >= 3 && parts[0] == "src" {
        let id = format!("src/{}", parts[1]);
        let name = parts[1].to_string();
        return (id, name);
    }
    if parts.len() >= 2 {
        let id = parts[0].to_string();
        let name = parts[0].to_string();
        return (id, name);
    }
    ("root".to_string(), "root".to_string())
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
