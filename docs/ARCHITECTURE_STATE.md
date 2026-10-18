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

## Issue #8 — `forge-git` Unified Diff Parser, Hotspot Detector & Blast-Radius Analyzer (COMPLETED)
- **Crate:** `crates/forge-git` (`src/diff.rs`, `src/hotspot.rs`, `src/impact.rs`)
- **Public Types & Functions (`forge_git::{diff, hotspot, impact}`):** `FileChange`, `HotspotEntry`, `ChangeImpactReport`, `parse_unified_diff(diff_text: &str) -> Result<Vec<FileChange>, ForgeError>`, `collect_working_tree_diff(repo_root: &Path) -> Result<String, ForgeError>`, `compute_hotspots_from_log(git_log_text: &str, top_n: usize) -> Vec<HotspotEntry>`, `compute_change_impact(changes: &[FileChange], modules: &[Module], test_files: &BTreeSet<String>) -> ChangeImpactReport`.
- **Invariants:** `compute_change_impact` traverses reverse `Module::imports` edges via BFS to separate `directly_affected_modules` from `transitively_affected_modules` and flags modified source files lacking matching test files.

## Issue #9 — `forge-rules` Deterministic Rule Engine & Built-in TS/JS Rules (COMPLETED)
- **Crate:** `crates/forge-rules` (`src/builtin.rs`, `src/engine.rs`)
- **Public Types & Functions (`forge_rules::{builtin, engine}`):** `RuleConfig`, `RuleEngine` (`new`, `with_recommended_defaults`, `rules`, `evaluate`, plus `Analyzer` trait implementation), `RULE_NO_DIRECT_DB_IN_UI`, `RULE_NO_CIRCULAR_DEPS`, `RULE_REQUIRE_SERVICE_TESTS`, `RULE_MAX_FILE_LINES`.
- **Invariants:** `RuleEngine::evaluate` returns findings sorted deterministically by `(Severity::desc, id::asc)` across the 4 built-in rules (`arch/no-direct-db-in-ui`, `arch/no-circular-dependencies`, `quality/require-service-tests`, `quality/max-file-lines`).

## Issue #10 — `forge-security` Secret Scanner & Unsafe JS/TS Pattern Detector (COMPLETED)
- **Crate:** `crates/forge-security` (`src/secrets.rs`, `src/patterns.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_security`):** `SecurityScanner` (`scan_file_content(relative_path: &str, content: &str) -> Vec<Finding>`, `scan_repository_files(repo_root: &Path, files: &[FileNode]) -> Result<Vec<Finding>, ForgeError>`), `scan_secrets`, `scan_unsafe_patterns`.
- **Invariants:** All secret findings (`security/hardcoded-secret`, `security/private-key`) strictly redact credential values with `****REDACTED****` in `Finding::evidence`; unsafe pattern rules cover `security/no-eval`, `security/no-implied-eval`, `security/no-dangerous-inner-html`, `security/unsafe-shell-exec`, and `security/permissive-cors`.

## Issue #11 — `forge-core` 8-Dimension Health Scoring & Change Risk Engine (COMPLETED)
- **Crate:** `crates/forge-core` (`src/scoring.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_core::scoring`):** `RiskLevel`, `RiskReport`, `calculate_health_score(repo: &Repository) -> HealthScore`, `calculate_change_risk(affected_modules_count: usize, missing_tests_count: usize, diff_findings: &[Finding]) -> RiskReport`.
- **Invariants:** `calculate_health_score` computes all 8 weighted dimensions (`architecture` 0.18, `maintainability` 0.16, `reliability` 0.14, `security` 0.18, `testing` 0.12, `dependency_health` 0.10, `documentation` 0.06, `release_readiness` 0.06) in `BTreeMap` with explicit penalty reasons; `calculate_change_risk` deterministically maps risk score to `Low|Medium|High|Critical`.

## Issue #12 — `forge-cli` CLI Parser, Exit Codes & Report Formatters (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/cli.rs`, `src/exit_code.rs`, `src/formatters/{mod.rs,terminal.rs,json.rs,markdown.rs,sarif.rs}`)
- **Public Types & Functions (`forge_cli`):** `Cli`, `Commands`, `OutputFormat` (`Terminal`, `Json`, `Markdown`, `Sarif`, `Github`), `ExitCode` (`Success = 0`, `FindingsAboveThreshold = 1`, `ConfigurationError = 2`, `ExecutionError = 3`, `SecurityPolicyFailure = 4`), `parse_severity_threshold`, `render_repository_report(repo: &Repository, format: OutputFormat) -> Result<String, ForgeError>`.
- **Invariants:** `render_repository_report` emits deterministic OASIS SARIF v2.1.0 JSON (`https://json.schemastore.org/sarif-2.1.0.json`), GitHub Actions workflow annotations (`::error`, `::warning`, `::notice`), Markdown tables, pretty-printed JSON, and terminal summaries.

## Issue #13 — `forge-cli` `eforge init` & `eforge doctor` Commands (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/commands/{mod.rs,init.rs,doctor.rs}`)
- **Public Types & Functions (`forge_cli::commands::{init, doctor}`):** `DoctorCheck`, `DoctorReport`, `run_init(repo_root: &Path) -> Result<PathBuf, ForgeError>`, `run_doctor(repo_root: &Path) -> Result<DoctorReport, ForgeError>`, `render_doctor_report(report: &DoctorReport, format: OutputFormat) -> Result<String, ForgeError>`.
- **Invariants:** `run_init` initializes `.eforge/` and writes a starter `eforge.config.yaml` without overwriting existing configs; `run_doctor` completes in `< 5s` evaluating repository inventory, language/framework detection, `package.json`/lockfile hygiene, test suite presence, and security/secret hygiene.
