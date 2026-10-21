//! `forge-analyzer` — Project discovery, AST/import graph, and language/framework detectors.

pub mod composer_manifest;
pub mod dep_health;
pub mod detector;
pub mod go_imports;
pub mod go_manifest;
pub mod graph;
pub mod package_json;
pub mod python_imports;
pub mod python_manifest;
pub mod rust_imports;
pub mod rust_manifest;
pub mod scanner;
pub mod ts_imports;

pub use composer_manifest::{
    analyze_composer_dependencies, is_composer_unpinned, parse_composer_json_content,
    parse_composer_manifest, read_composer_lock_versions, ParsedComposerManifest,
    PHP_IMPLICIT_RUNNERS,
};
pub use dep_health::{
    analyze_npm_dependencies, analyze_polyglot_dependencies, analyze_python_dependencies,
    DependencyReport,
};
pub use detector::{detect_ecosystem, detect_language, is_test_file, EcosystemSignals};
pub use forge_core::Analyzer;
pub use go_imports::{
    extract_go_external_module, extract_go_imports, is_go_stdlib, resolve_go_import,
    strip_go_comments, GO_STDLIB_ROOT_PACKAGES,
};
pub use go_manifest::{parse_go_manifest, parse_go_mod_content, ParsedGoManifest};
pub use graph::{
    build_go_architecture_graph, build_polyglot_architecture_graph,
    build_python_architecture_graph, build_rust_architecture_graph, build_ts_architecture_graph,
    infer_rust_module_boundary, ArchitectureGraph,
};
pub use package_json::{parse_package_json, ParsedManifest};
pub use python_imports::{
    extract_pypi_package_name, extract_python_imports_and_exports, is_python_stdlib,
    resolve_python_import, strip_python_comments, PYTHON_STDLIB_MODULES,
};
pub use python_manifest::{
    is_python_unpinned, normalize_python_package_name, parse_pyproject_toml_content,
    parse_python_manifests, parse_requirement_line, parse_requirements_txt_content,
    read_python_lockfile_versions, ParsedPythonManifest,
};
pub use rust_imports::{
    expand_use_tree, extract_rust_external_crate, extract_rust_mods_and_uses, is_rust_std,
    resolve_rust_import, strip_rust_comments, RUST_STD_CRATES,
};
pub use rust_manifest::{
    parse_cargo_dependencies, parse_cargo_toml_content, parse_rust_workspace,
    read_cargo_lock_versions, ParsedCargoDeps, ParsedRustWorkspace,
};
pub use scanner::{scan_repository, DiscoverySummary};
pub use ts_imports::{
    extract_npm_package_name, extract_sfc_script_content, extract_ts_imports_and_exports,
    resolve_import_specifier,
};
