//! `forge-analyzer` — Project discovery, AST/import graph, and language/framework detectors.

pub mod dep_health;
pub mod detector;
pub mod graph;
pub mod package_json;
pub mod python_manifest;
pub mod scanner;
pub mod ts_imports;

pub use dep_health::{analyze_npm_dependencies, DependencyReport};
pub use detector::{detect_ecosystem, detect_language, is_test_file, EcosystemSignals};
pub use forge_core::Analyzer;
pub use graph::{build_ts_architecture_graph, ArchitectureGraph};
pub use package_json::{parse_package_json, ParsedManifest};
pub use python_manifest::{
    is_python_unpinned, normalize_python_package_name, parse_pyproject_toml_content,
    parse_python_manifests, parse_requirement_line, parse_requirements_txt_content,
    read_python_lockfile_versions, ParsedPythonManifest,
};
pub use scanner::{scan_repository, DiscoverySummary};
pub use ts_imports::{
    extract_npm_package_name, extract_sfc_script_content, extract_ts_imports_and_exports,
    resolve_import_specifier,
};
