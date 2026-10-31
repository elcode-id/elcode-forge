//! In-memory virtual refactoring simulator (`eforge simulate`).
//!
//! Evaluates the structural impact of moving files/modules or adding/removing import edges
//! in memory before touching disk, reporting resolved vs. newly introduced Tarjan SCC cycles
//! and relative import specifier rewrites.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use forge_core::{FileImportEdge, FileNode, ForgeError, Module};
use serde::{Deserialize, Serialize};

use crate::graph::{compute_high_centrality, detect_circular_dependencies, ArchitectureGraph};
use crate::incremental::{analyze_architecture_incremental, infer_boundary_for_lang, FileAstCache};

/// A single virtual refactoring operation evaluated in memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum RefactorOperation {
    MoveFile {
        from: String,
        to: String,
    },
    MoveModule {
        from_prefix: String,
        to_prefix: String,
    },
    AddImportEdge {
        from_file: String,
        to_file: String,
    },
    RemoveImportEdge {
        from_file: String,
        to_file: String,
    },
}

/// Declarative refactoring plan file schema (`eforge simulate --plan <file.yaml>`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefactorPlanFile {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub operations: Vec<RefactorOperation>,
}

/// Preview of an import specifier that must be rewritten when a file or module moves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRewritePreview {
    pub importer_file: String,
    pub line: u32,
    pub old_specifier: String,
    pub new_specifier: String,
}

/// Complete before/after report produced by [`simulate_refactor`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationReport {
    pub operations: Vec<RefactorOperation>,
    pub baseline_module_count: usize,
    pub simulated_module_count: usize,
    pub baseline_cycles: Vec<Vec<String>>,
    pub simulated_cycles: Vec<Vec<String>>,
    pub resolved_cycles: Vec<Vec<String>>,
    pub introduced_cycles: Vec<Vec<String>>,
    pub import_rewrites: Vec<ImportRewritePreview>,
    pub simulated_modules: Vec<Module>,
    pub is_safe_to_apply: bool,
    pub summary: String,
}

/// Parses a YAML or JSON refactoring plan specification into a [`RefactorPlanFile`].
pub fn parse_refactor_plan(content: &str) -> Result<RefactorPlanFile, ForgeError> {
    let trimmed = content.trim();
    if trimmed.starts_with('{') {
        serde_json::from_str::<RefactorPlanFile>(trimmed).map_err(|e| ForgeError::Config {
            message: format!("invalid JSON refactor plan: {e}"),
        })
    } else {
        // Parse compact YAML plan deterministically
        parse_simple_plan_yaml(trimmed)
    }
}

/// Executes `operations` purely in memory against the repository's [`ArchitectureGraph`] and
/// file-level import edges, returning a [`SimulationReport`] with zero disk mutations.
pub fn simulate_refactor(
    repo_root: &Path,
    files: &[FileNode],
    operations: &[RefactorOperation],
) -> Result<SimulationReport, ForgeError> {
    let mut cache = FileAstCache::load(repo_root);
    let baseline = analyze_architecture_incremental(repo_root, files, &mut cache)?;

    let mut path_remap: BTreeMap<String, String> = BTreeMap::new();
    for file in files {
        let mut mapped = file.relative_path.clone();
        for op in operations {
            match op {
                RefactorOperation::MoveFile { from, to } => {
                    let norm_from = normalize_rel(from);
                    let norm_to = normalize_rel(to);
                    if mapped == norm_from {
                        mapped = norm_to;
                    }
                }
                RefactorOperation::MoveModule {
                    from_prefix,
                    to_prefix,
                } => {
                    let norm_from = normalize_rel(from_prefix);
                    let norm_to = normalize_rel(to_prefix);
                    let prefix_slash = format!("{norm_from}/");
                    if mapped == norm_from {
                        mapped = norm_to;
                    } else if let Some(rest) = mapped.strip_prefix(&prefix_slash) {
                        mapped = format!("{norm_to}/{rest}");
                    }
                }
                RefactorOperation::AddImportEdge { .. }
                | RefactorOperation::RemoveImportEdge { .. } => {}
            }
        }
        path_remap.insert(file.relative_path.clone(), mapped);
    }

    let mut import_rewrites = Vec::new();
    let mut simulated_edges: Vec<FileImportEdge> = Vec::new();

    let removed_pairs: BTreeSet<(String, String)> = operations
        .iter()
        .filter_map(|op| match op {
            RefactorOperation::RemoveImportEdge { from_file, to_file } => {
                Some((normalize_rel(from_file), normalize_rel(to_file)))
            }
            _ => None,
        })
        .collect();

    for edge in &baseline.file_edges {
        if removed_pairs.contains(&(edge.from_file.clone(), edge.to_file.clone())) {
            continue;
        }
        let new_from = path_remap
            .get(&edge.from_file)
            .cloned()
            .unwrap_or_else(|| edge.from_file.clone());
        let new_to = path_remap
            .get(&edge.to_file)
            .cloned()
            .unwrap_or_else(|| edge.to_file.clone());

        if new_from != edge.from_file || new_to != edge.to_file {
            let new_spec = compute_relative_import_specifier(&new_from, &new_to);
            if new_spec != edge.raw_specifier {
                import_rewrites.push(ImportRewritePreview {
                    importer_file: new_from.clone(),
                    line: edge.line,
                    old_specifier: edge.raw_specifier.clone(),
                    new_specifier: new_spec.clone(),
                });
            }
        }

        let lang = infer_lang_from_ext(&new_from);
        let (from_mod, _) = infer_boundary_for_lang(&new_from, lang);
        let (to_mod, _) = infer_boundary_for_lang(&new_to, lang);

        simulated_edges.push(FileImportEdge {
            from_file: new_from,
            to_file: new_to,
            from_module: from_mod,
            to_module: to_mod,
            raw_specifier: edge.raw_specifier.clone(),
            line: edge.line,
            column: edge.column,
        });
    }

    // Apply virtual AddImportEdge operations
    for op in operations {
        if let RefactorOperation::AddImportEdge { from_file, to_file } = op {
            let nf = normalize_rel(from_file);
            let nt = normalize_rel(to_file);
            let lang = infer_lang_from_ext(&nf);
            let (from_mod, _) = infer_boundary_for_lang(&nf, lang);
            let (to_mod, _) = infer_boundary_for_lang(&nt, lang);
            simulated_edges.push(FileImportEdge {
                from_file: nf.clone(),
                to_file: nt.clone(),
                from_module: from_mod,
                to_module: to_mod,
                raw_specifier: compute_relative_import_specifier(&nf, &nt),
                line: 1,
                column: 1,
            });
        }
    }

    let simulated_graph = build_simulated_module_graph(
        files,
        &path_remap,
        &simulated_edges,
        &baseline.graph.external_imports,
    );

    let base_cycle_set: BTreeSet<Vec<String>> = baseline
        .graph
        .circular_dependencies
        .iter()
        .cloned()
        .collect();
    let sim_cycle_set: BTreeSet<Vec<String>> = simulated_graph
        .circular_dependencies
        .iter()
        .cloned()
        .collect();

    let resolved_cycles: Vec<Vec<String>> =
        base_cycle_set.difference(&sim_cycle_set).cloned().collect();
    let introduced_cycles: Vec<Vec<String>> =
        sim_cycle_set.difference(&base_cycle_set).cloned().collect();

    let is_safe_to_apply = introduced_cycles.is_empty();
    let summary = format!(
        "Simulated {} operation(s): {} module(s) -> {} module(s), {} cycle(s) resolved, {} new cycle(s) introduced, {} import rewrite(s) required.",
        operations.len(),
        baseline.graph.modules.len(),
        simulated_graph.modules.len(),
        resolved_cycles.len(),
        introduced_cycles.len(),
        import_rewrites.len()
    );

    Ok(SimulationReport {
        operations: operations.to_vec(),
        baseline_module_count: baseline.graph.modules.len(),
        simulated_module_count: simulated_graph.modules.len(),
        baseline_cycles: baseline.graph.circular_dependencies,
        simulated_cycles: simulated_graph.circular_dependencies,
        resolved_cycles,
        introduced_cycles,
        import_rewrites,
        simulated_modules: simulated_graph.modules,
        is_safe_to_apply,
        summary,
    })
}

fn build_simulated_module_graph(
    files: &[FileNode],
    path_remap: &BTreeMap<String, String>,
    edges: &[FileImportEdge],
    external_imports: &BTreeSet<String>,
) -> ArchitectureGraph {
    #[derive(Default)]
    struct SimModAcc {
        name: String,
        root_path: String,
        files: BTreeSet<String>,
        imports: BTreeSet<String>,
        exports: BTreeSet<String>,
    }

    let mut module_map: BTreeMap<String, SimModAcc> = BTreeMap::new();

    for file in files {
        let mapped = path_remap
            .get(&file.relative_path)
            .cloned()
            .unwrap_or_else(|| file.relative_path.clone());
        let lang = infer_lang_from_ext(&mapped);
        if lang == "unknown" {
            continue;
        }
        let (mod_id, mod_name) = infer_boundary_for_lang(&mapped, lang);
        let acc = module_map
            .entry(mod_id.clone())
            .or_insert_with(|| SimModAcc {
                name: mod_name,
                root_path: mod_id.clone(),
                ..Default::default()
            });
        acc.files.insert(mapped);
    }

    for edge in edges {
        if edge.from_module != edge.to_module {
            if let Some(acc) = module_map.get_mut(&edge.from_module) {
                acc.imports.insert(edge.to_module.clone());
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

    ArchitectureGraph {
        modules,
        circular_dependencies,
        high_centrality_modules,
        external_imports: external_imports.clone(),
    }
}

fn normalize_rel(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

fn infer_lang_from_ext(path: &str) -> &'static str {
    if path.ends_with(".ts") || path.ends_with(".tsx") {
        "typescript"
    } else if path.ends_with(".js") || path.ends_with(".jsx") {
        "javascript"
    } else if path.ends_with(".vue") {
        "vue"
    } else if path.ends_with(".svelte") {
        "svelte"
    } else if path.ends_with(".py") {
        "python"
    } else if path.ends_with(".go") {
        "go"
    } else if path.ends_with(".rs") {
        "rust"
    } else {
        "unknown"
    }
}

/// Computes the normalized relative import specifier from `importer_file` to `target_file`.
#[must_use]
#[allow(clippy::same_item_push)]
pub fn compute_relative_import_specifier(importer_file: &str, target_file: &str) -> String {
    let from_parts: Vec<&str> = importer_file.split('/').collect();
    let to_parts: Vec<&str> = target_file.split('/').collect();

    let from_dir = if from_parts.len() > 1 {
        &from_parts[..from_parts.len() - 1]
    } else {
        &[][..]
    };
    let to_dir = if to_parts.len() > 1 {
        &to_parts[..to_parts.len() - 1]
    } else {
        &[][..]
    };
    let target_filename = to_parts.last().copied().unwrap_or(target_file);
    let target_stem = strip_code_extension(target_filename);

    let mut common = 0;
    while common < from_dir.len() && common < to_dir.len() && from_dir[common] == to_dir[common] {
        common += 1;
    }

    let up_count = from_dir.len() - common;
    let mut rel_parts: Vec<&str> = Vec::new();
    if up_count == 0 {
        rel_parts.push(".");
    } else {
        for _ in 0..up_count {
            rel_parts.push("..");
        }
    }
    for seg in &to_dir[common..] {
        rel_parts.push(seg);
    }
    rel_parts.push(target_stem);
    rel_parts.join("/")
}

fn strip_code_extension(filename: &str) -> &str {
    for ext in [".tsx", ".ts", ".jsx", ".js", ".py", ".rs", ".go"] {
        if let Some(stem) = filename.strip_suffix(ext) {
            return stem;
        }
    }
    filename
}

fn parse_simple_plan_yaml(content: &str) -> Result<RefactorPlanFile, ForgeError> {
    let mut description = None;
    let mut operations = Vec::new();
    let mut current_op: Option<String> = None;
    let mut arg1: Option<String> = None;
    let mut arg2: Option<String> = None;

    let flush_op = |op_kind: &mut Option<String>,
                    a1: &mut Option<String>,
                    a2: &mut Option<String>,
                    ops: &mut Vec<RefactorOperation>| {
        if let (Some(kind), Some(v1), Some(v2)) = (op_kind.take(), a1.take(), a2.take()) {
            match kind.as_str() {
                "move_file" | "mv" => ops.push(RefactorOperation::MoveFile { from: v1, to: v2 }),
                "move_module" => ops.push(RefactorOperation::MoveModule {
                    from_prefix: v1,
                    to_prefix: v2,
                }),
                "add_import_edge" | "add_import" => ops.push(RefactorOperation::AddImportEdge {
                    from_file: v1,
                    to_file: v2,
                }),
                "remove_import_edge" | "remove_import" => {
                    ops.push(RefactorOperation::RemoveImportEdge {
                        from_file: v1,
                        to_file: v2,
                    });
                }
                _ => {}
            }
        }
    };

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(desc) = line.strip_prefix("description:") {
            description = Some(unquote(desc));
            continue;
        }
        if line.starts_with("- op:") || line.starts_with("op:") {
            flush_op(&mut current_op, &mut arg1, &mut arg2, &mut operations);
            let val = line
                .trim_start_matches('-')
                .trim()
                .strip_prefix("op:")
                .unwrap_or("");
            current_op = Some(unquote(val));
            continue;
        }
        if let Some(val) = line
            .strip_prefix("from:")
            .or_else(|| line.strip_prefix("from_prefix:"))
            .or_else(|| line.strip_prefix("from_file:"))
        {
            arg1 = Some(unquote(val));
            continue;
        }
        if let Some(val) = line
            .strip_prefix("to:")
            .or_else(|| line.strip_prefix("to_prefix:"))
            .or_else(|| line.strip_prefix("to_file:"))
        {
            arg2 = Some(unquote(val));
        }
    }
    flush_op(&mut current_op, &mut arg1, &mut arg2, &mut operations);

    if operations.is_empty() {
        return Err(ForgeError::Config {
            message: "refactor plan contains no valid operations".to_string(),
        });
    }

    Ok(RefactorPlanFile {
        description,
        operations,
    })
}

fn unquote(raw: &str) -> String {
    raw.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::scan_repository;
    use std::path::PathBuf;

    #[test]
    fn simulates_moving_file_and_breaking_cycle_in_memory() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app");

        let discovery = scan_repository(&fixture, &[]);
        assert!(discovery.is_ok());
        let Ok(summary) = discovery else { return };

        // Co-locating client.ts into src/services resolves the inter-module cycle between src/db and src/services
        let ops = vec![RefactorOperation::MoveFile {
            from: "src/db/client.ts".to_string(),
            to: "src/services/client.ts".to_string(),
        }];

        let sim = simulate_refactor(&fixture, &summary.files, &ops);
        assert!(sim.is_ok());
        let Ok(report) = sim else { return };

        assert_eq!(report.baseline_cycles.len(), 1);
        assert_eq!(report.simulated_cycles.len(), 0);
        assert_eq!(report.resolved_cycles.len(), 1);
        assert!(report.introduced_cycles.is_empty());
        assert!(report.is_safe_to_apply);
        assert!(!report.import_rewrites.is_empty());
    }
}
