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

