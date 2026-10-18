# ARCHITECTURE_STATE.md — ELCODE Forge Living Handover Memory

## Issue #1 — `forge-core` & Workspace Bootstrap (COMPLETED)
- **Crate:** `crates/forge-core` (Root DAG Leaf Crate) + Workspace (`Cargo.toml`)
- **Public Types Locked (`forge_core::ir`):** `Severity`, `Confidence`, `SourceLocation`, `Finding`, `FileNode`, `Module`, `Dependency`, `DimensionScore`, `HealthScore`, `Repository`.
- **Public Error & Traits (`forge_core::{error, traits}`):** `ForgeError` (`Io`, `PathTraversal`, `Config`, `Serialization`, `Analysis`), `Analyzer` trait (`fn name(&self) -> &'static str`, `fn analyze(&self, repo: &Repository) -> Result<Vec<Finding>, ForgeError>`).
- **CLI Binary Entrypoints (`crates/forge-cli`):** Primary binary `eforge` (`src/main.rs`) and secondary alias `elcode-forge` (`src/bin/elcode-forge.rs`).
- **Invariants:** `forge-core` has zero internal crate dependencies; workspace enforces `unsafe_code = "forbid"` and denies `unwrap_used`, `expect_used`, `panic`, `todo`, and `unimplemented`.

## Issue #2 — `forge-core` Config Loader & Canonical Path Guard (COMPLETED)
- **Crate:** `crates/forge-core` (`src/config.rs`, `src/path_guard.rs`)
- **Public Types & Functions (`forge_core::{config, path_guard}`):** `ForgeConfig`, `PRIMARY_CONFIG_FILE` (`"eforge.config.yaml"`), `FALLBACK_CONFIG_FILE` (`"forge.config.yaml"`), `load_config(repo_root: &Path) -> Result<ForgeConfig, ForgeError>`, `parse_config_yaml(content: &str) -> Result<ForgeConfig, ForgeError>`, `resolve_safe_path(repo_root: &Path, candidate: &Path) -> Result<PathBuf, ForgeError>`.
- **Invariants:** `load_config` prioritizes `eforge.config.yaml` over `forge.config.yaml`; `resolve_safe_path` enforces canonical path boundary checks and rejects any `..` or symlink escape outside `repo_root` with `ForgeError::PathTraversal`.

## Issue #3 — `forge-core` Incremental `.eforge/` Cache & BLAKE3 Hashing (COMPLETED)
- **Crate:** `crates/forge-core` (`src/cache.rs`)
- **Public Types & Functions (`forge_core::cache`):** `EFORGE_DIR` (`".eforge"`), `CacheMetadata`, `ForgeCache` (`init`, `root_dir`, `cache_dir`, `index_dir`, `metadata_path`, `hash_bytes`, `load_metadata`, `save_metadata`, `diff_changed_files`).
- **Invariants:** Local cache state always resides in `<repo_root>/.eforge/` (`.eforge/cache/`, `.eforge/index/`, `.eforge/metadata.json`); `diff_changed_files` invalidates all files when `config_hash` or `analyzer_version` changes.

## Issue #4 — Path-Aware Rust CI Pipeline & Curated TS/JS Fixtures (COMPLETED)
- **Paths:** `.github/workflows/ci-rust.yml`, `fixtures/next-app/`, `fixtures/vulnerable-ts-app/`
- **Capabilities:** Path-filtered CI workflow with `Swatinem/rust-cache@v2`, `cargo-nextest`, and selective per-crate PR verification; deterministic Next.js (`fixtures/next-app`) and security (`fixtures/vulnerable-ts-app`) test repositories.
- **Invariants:** `fixtures/next-app` intentionally contains 1 UI->DB layer violation (`UserCard.tsx` -> `db/client.ts`), 1 circular dependency (`services/userService.ts` <-> `db/client.ts`), 1 unused dependency (`unused-analytics-sdk`), and 1 wildcard dependency (`legacy-unpinned-pkg`); do not remove these fixture signals as downstream tests rely on them.

## Issue #5 — `forge-analyzer` Streaming Repository Scanner & Ecosystem Detector (COMPLETED)
- **Crate:** `crates/forge-analyzer` (`src/scanner.rs`, `src/detector.rs`)
- **Public Types & Functions (`forge_analyzer::{scanner, detector}`):** `DiscoverySummary`, `EcosystemSignals`, `scan_repository(repo_root: &Path, extra_ignores: &[String]) -> Result<DiscoverySummary, ForgeError>`, `detect_ecosystem`, `detect_language`, `is_test_file`.
- **Invariants:** `scan_repository` normalizes all `FileNode::relative_path` values to forward slashes (`/`), sorts files deterministically by `relative_path`, and skips `.git`, `.eforge`, `node_modules`, `target`, `dist`, `.next`, `.nuxt`, `.svelte-kit`.

## Issue #6 — `forge-analyzer` `package.json` Parser & Dependency Health Analyzer (COMPLETED)
- **Crate:** `crates/forge-analyzer` (`src/package_json.rs`, `src/dep_health.rs`)
- **Public Types & Functions (`forge_analyzer::{package_json, dep_health}`):** `ParsedManifest`, `DependencyReport`, `parse_package_json(repo_root: &Path) -> Result<ParsedManifest, ForgeError>`, `analyze_npm_dependencies(repo_root: &Path, imported_packages: &BTreeSet<String>) -> Result<DependencyReport, ForgeError>`.
- **Invariants:** `parse_package_json` sorts dependencies deterministically by `(name, is_dev)` and resolves versions from `package-lock.json` when present; `analyze_npm_dependencies` emits findings for `deps/unpinned-version`, `deps/unused-dependency`, and `deps/duplicate-dependency`.

## Issue #7 — `forge-analyzer` TS/JS Import Graph & Cycle Detection (COMPLETED)
- **Crate:** `crates/forge-analyzer` (`src/ts_imports.rs`, `src/graph.rs`)
- **Public Types & Functions (`forge_analyzer::{ts_imports, graph}`):** `ArchitectureGraph`, `build_ts_architecture_graph(repo_root: &Path, files: &[FileNode]) -> Result<ArchitectureGraph, ForgeError>`, `extract_ts_imports_and_exports`, `resolve_import_specifier`, `extract_npm_package_name`.
- **Invariants:** Logical `Module` boundaries use normalized directory paths (`id == root_path`, e.g., `"src/components"`, `"src/db"`, `"src/services"`, `"src/pages"`), and `Module::imports` stores sorted target module IDs; Tarjan's SCC algorithm detects cycles deterministically.






