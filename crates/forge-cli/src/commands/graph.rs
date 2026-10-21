//! `eforge graph` command — exports module architecture graph as Terminal, JSON, or Mermaid Markdown.

use std::path::Path;

use forge_analyzer::{
    build_go_architecture_graph, build_python_architecture_graph, build_rust_architecture_graph,
    build_ts_architecture_graph, parse_go_manifest, scanner::scan_repository,
};
use forge_core::{config::load_config, ForgeError};

use crate::cli::OutputFormat;

/// Builds the [`forge_analyzer::ArchitectureGraph`] for `repo_root` and renders it in `format`.
pub fn run_graph(repo_root: &Path, format: OutputFormat) -> Result<String, ForgeError> {
    let config = load_config(repo_root)?;
    let discovery = scan_repository(repo_root, &config.ignore_patterns)?;

    let graph = if discovery.languages.contains(&"python".to_string())
        || discovery
            .files
            .iter()
            .any(|f| f.relative_path.ends_with(".py"))
    {
        build_python_architecture_graph(repo_root, &discovery.files)?
    } else if discovery.languages.contains(&"go".to_string())
        || discovery
            .files
            .iter()
            .any(|f| f.relative_path.ends_with(".go"))
    {
        let module_path = if repo_root.join("go.mod").is_file() {
            parse_go_manifest(repo_root)
                .ok()
                .and_then(|m| m.module_path)
        } else {
            None
        };
        build_go_architecture_graph(repo_root, &discovery.files, module_path.as_deref())?
    } else if discovery.languages.contains(&"rust".to_string())
        || discovery
            .files
            .iter()
            .any(|f| f.relative_path.ends_with(".rs"))
    {
        build_rust_architecture_graph(repo_root, &discovery.files)?
    } else {
        build_ts_architecture_graph(repo_root, &discovery.files)?
    };

    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(&graph).map_err(|err| ForgeError::Serialization {
                message: err.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut md = String::new();
            md.push_str("# ELCODE Forge Architecture Graph\n\n");
            md.push_str("```mermaid\nflowchart TD\n");
            for (idx, module) in graph.modules.iter().enumerate() {
                md.push_str(&format!("    M{idx}[\"{}\"]\n", module.id));
            }
            for (src_idx, module) in graph.modules.iter().enumerate() {
                for target_id in &module.imports {
                    if let Some(dst_idx) = graph.modules.iter().position(|m| &m.id == target_id) {
                        md.push_str(&format!("    M{src_idx} --> M{dst_idx}\n"));
                    }
                }
            }
            md.push_str("```\n\n");

            if !graph.circular_dependencies.is_empty() {
                md.push_str("## Circular Dependencies\n\n");
                for cycle in &graph.circular_dependencies {
                    md.push_str(&format!("- `{}`\n", cycle.join(" <-> ")));
                }
            }
            Ok(md)
        }
        OutputFormat::Terminal => {
            let mut out = String::new();
            out.push_str("ELCODE Forge Architecture Graph\n");
            out.push_str("-------------------------------\n");
            out.push_str(&format!("Modules ({}):\n", graph.modules.len()));
            for module in &graph.modules {
                let targets = if module.imports.is_empty() {
                    "none".to_string()
                } else {
                    module.imports.join(", ")
                };
                out.push_str(&format!(
                    "  - {} ({} file(s)) -> imports: [{}]\n",
                    module.id,
                    module.files.len(),
                    targets
                ));
            }
            if !graph.circular_dependencies.is_empty() {
                out.push_str("\nCircular Dependencies:\n");
                for cycle in &graph.circular_dependencies {
                    out.push_str(&format!("  ! {}\n", cycle.join(" <-> ")));
                }
            }
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::run_graph;
    use crate::cli::OutputFormat;

    fn fixture_next_app_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app")
    }

    #[test]
    fn renders_mermaid_and_json_architecture_graph_for_fixture_next_app() {
        let root = fixture_next_app_dir();
        let md_res = run_graph(&root, OutputFormat::Markdown);
        assert!(md_res.is_ok());
        if let Ok(md) = md_res {
            assert!(md.contains("```mermaid"));
            assert!(md.contains("flowchart TD"));
            assert!(md.contains("src/components"));
            assert!(md.contains("src/db <-> src/services"));
        }

        let json_res = run_graph(&root, OutputFormat::Json);
        assert!(json_res.is_ok());
        if let Ok(json_text) = json_res {
            assert!(json_text.contains("\"circular_dependencies\""));
        }
    }
}
