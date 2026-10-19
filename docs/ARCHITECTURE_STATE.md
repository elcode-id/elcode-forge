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

## Issue #14 — `forge-cli` `eforge scan` Command Pipeline (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/commands/scan.rs`, `src/commands/mod.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_cli::commands::scan`):** `ScanOptions`, `execute_scan(opts: &ScanOptions) -> Result<(Repository, ExitCode), ForgeError>`.
- **Invariants:** `execute_scan` orchestrates config loading, repository file discovery, TS/JS architecture graph & Tarjan cycle detection, npm dependency health analysis, `.eforge/metadata.json` BLAKE3 incremental hashing, built-in architectural/quality rules, secret/unsafe-pattern scanning, and 8-dimension health scoring with deterministic `(Severity::desc, id::asc)` finding ordering.

## Issue #15 — `forge-cli` `eforge review` & `eforge diff` Commands — Milestone 1 Complete (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/commands/{review.rs,diff.rs,mod.rs}`, `src/lib.rs`)
- **Public Types & Functions (`forge_cli::commands::{review, diff}`):** `ReviewReport`, `execute_review(repo_root: &Path, diff_input: Option<&str>, fast_mode: bool) -> Result<(ReviewReport, ExitCode), ForgeError>`, `render_review_report`, `execute_diff(repo_root: &Path, diff_input: Option<&str>) -> Result<ChangeImpactReport, ForgeError>`, `render_diff_report`.
- **Invariants:** All Phase 0 & Phase 1 milestone CLI commands (`eforge init`, `eforge doctor`, `eforge scan`, `eforge review`, `eforge diff`) are wired into both `eforge` and `elcode-forge` binaries with deterministic `ExitCode` (`0..=4`) policy evaluation.

## Issue #35 — `forge-core`, `forge-rules`, `forge-analyzer` Config Rule Overrides, Vue/Svelte SFC & `pnpm-lock.yaml` (COMPLETED)
- **Crates:** `crates/forge-core` (`src/config.rs`, `src/lib.rs`), `crates/forge-rules` (`src/engine.rs`), `crates/forge-analyzer` (`src/ts_imports.rs`, `src/graph.rs`, `src/package_json.rs`), `crates/forge-cli` (`src/commands/scan.rs`)
- **Public Types & Functions:** `RuleOverrideConfig`, `ForgeConfig::rules`, `RuleEngine::with_config_overrides(&BTreeMap<String, RuleOverrideConfig>)`, `extract_sfc_script_content(raw_sfc: &str) -> String`.
- **Invariants:** `parse_config_yaml` supports per-rule overrides (`enabled`, `severity`, `max_lines`/`threshold`); `build_ts_architecture_graph` extracts `<script>` blocks from `.vue` and `.svelte` files; `parse_package_json` resolves locked versions from `package-lock.json` and `pnpm-lock.yaml`.

## Issue #36 — `forge-cli` `eforge dependencies`, `eforge security` & `eforge graph` Subcommands (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/commands/{dependencies.rs,security.rs,graph.rs,mod.rs}`, `src/cli.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_cli::commands::{dependencies, security, graph}`):** `run_dependencies(repo_root: &Path, format: OutputFormat) -> Result<String, ForgeError>`, `run_security(repo_root: &Path, format: OutputFormat) -> Result<(String, ExitCode), ForgeError>`, `run_graph(repo_root: &Path, format: OutputFormat) -> Result<String, ForgeError>`.
- **Invariants:** `run_graph` renders Mermaid `flowchart TD` diagrams in Markdown format, full `ArchitectureGraph` JSON in JSON format, and module dependency lists in Terminal format.

## Issue #37 — `forge-cli` `eforge onboard` & `eforge docs` Documentation Generator (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/commands/onboard.rs`, `src/commands/mod.rs`, `src/cli.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_cli::commands::onboard`):** `GeneratedDocsBundle`, `generate_onboarding_docs(repo_root: &Path, output_dir: &Path) -> Result<GeneratedDocsBundle, ForgeError>`.
- **Invariants:** `generate_onboarding_docs` writes `architecture.md`, `modules.md`, `dependencies.md`, `onboarding.md`, and `risks.md` inside `output_dir` (defaulting to `<repo_root>/docs/forge`), each prefixed with an explicit `<!-- GENERATED BY ELCODE FORGE (eforge). DO NOT EDIT MANUALLY. -->` banner.

## Issue #38 — Composite GitHub Action (`elcode-id/forge-action`) & Self-Dogfooding Workflow (COMPLETED)
- **Files:** `integrations/github-action/action.yml`, `integrations/github-action/README.md`, `.github/workflows/eforge-dogfood.yml`
- **Action Surface:** Composite action accepting `command`, `path`, `base`, `format`, `strict`, `fail-on`, `sarif-output`, and `step-summary`, emitting `exit-code` and `sarif-file` outputs and writing Markdown summaries to `$GITHUB_STEP_SUMMARY`.
- **Invariants:** `.github/workflows/eforge-dogfood.yml` continuously runs `eforge doctor` and `eforge scan` against the ELCODE Forge workspace and invokes `./integrations/github-action` against `fixtures/next-app`.

## Issue #39 — `forge-mcp` Untrusted Data Envelope & Prompt-Injection Sanitizer (COMPLETED)
- **Crate:** `crates/forge-mcp` (`src/boundary.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_mcp`):** `UntrustedEnvelope<T>`, `UNTRUSTED_REPO_DATA_LEVEL`, `UNTRUSTED_SECURITY_NOTICE`, `wrap_untrusted_payload<T: Serialize>(payload: T, raw_texts: &[&str]) -> UntrustedEnvelope<T>`, `detect_prompt_injection_signals(text: &str) -> Vec<String>`.
- **Invariants:** Every MCP tool response payload is wrapped inside `UntrustedEnvelope<T>` with `trust_level = "untrusted_repository_data"` and deterministic prompt-injection signal detection.

## Issue #40 — `forge-mcp` `eforge_*` MCP Tool Registry & Deterministic Handlers (COMPLETED)
- **Crate:** `crates/forge-mcp` (`src/tools.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_mcp`):** `McpToolDescriptor`, `list_mcp_tools() -> Vec<McpToolDescriptor>`, `call_mcp_tool(repo_root: &Path, tool_name: &str, arguments: &Value) -> Result<Value, ForgeError>`.
- **Invariants:** Exposes 6 deterministic tools in the `eforge_*` namespace (`eforge_project_info`, `eforge_get_architecture`, `eforge_get_dependencies`, `eforge_get_findings`, `eforge_get_change_impact`, `eforge_explain_module`), each wrapped in `UntrustedEnvelope<T>`.

## Issue #41 — `forge-mcp` & `forge-cli` JSON-RPC 2.0 Stdio MCP Server (`eforge mcp`) (COMPLETED)
- **Crates:** `crates/forge-mcp` (`src/server.rs`, `src/lib.rs`), `crates/forge-cli` (`src/commands/mcp.rs`, `src/commands/mod.rs`, `src/cli.rs`, `src/lib.rs`)
- **Public Functions (`forge_mcp` / `forge_cli`):** `handle_jsonrpc_line(repo_root: &Path, raw_line: &str) -> Option<String>`, `serve_mcp_stdio(repo_root: &Path) -> Result<(), ForgeError>`, `run_mcp(repo_root: &Path) -> Result<(), ForgeError>`, `Commands::Mcp`.
- **Invariants:** `handle_jsonrpc_line` implements JSON-RPC 2.0 (`initialize`, `notifications/initialized`, `ping`, `tools/list`, `tools/call`) with protocol version `"2024-11-05"` and server name `"eforge-mcp"`.

## Issue #42 — `forge-cli` `eforge explain [target]` Command — Phase 2 Complete (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/commands/explain.rs`, `src/commands/mod.rs`, `src/cli.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_cli::commands::explain`):** `ExplainReport`, `execute_explain(repo_root: &Path, target_module_or_file: Option<&str>) -> Result<ExplainReport, ForgeError>`, `render_explain_report(report: &ExplainReport, format: OutputFormat) -> Result<String, ForgeError>`, `Commands::Explain`.
- **Invariants:** Deterministically resolves matching files, outgoing `imports`, incoming `imported_by`, `exports`, and `related_findings` for either the entire repository (`None` / `"."`) or a specific module/file path prefix.

## Issue #56 — `forge-git` Commit Log & HEAD Revision Collection (COMPLETED)
- **Crate:** `crates/forge-git` (`src/log.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_git`):** `GitRevision`, `read_head_revision(repo_root: &Path) -> Result<Option<GitRevision>, ForgeError>`, `collect_commit_log(repo_root: &Path, max_commits: usize) -> Result<String, ForgeError>`.
- **Invariants:** Invokes `git` with fixed argument vectors without shell evaluation; gracefully returns `Ok(None)` or empty log text if `repo_root` is not a Git repository or `git` fails.

## Issue #57 — `forge-cli` Deterministic `ReportBundle` and `eforge report` Command (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/commands/report.rs`, `src/cli.rs`, `src/commands/mod.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_cli::commands::report`):** `REPORT_SCHEMA_VERSION`, `ReportBundle`, `ArchitectureSummary`, `ReportOptions`, `build_report_bundle(repo_root: &Path, opts: &ReportOptions) -> Result<ReportBundle, ForgeError>`, `run_report(repo_root: &Path, output: Option<&Path>, hotspots_top_n: usize, no_git: bool) -> Result<String, ForgeError>`, `Commands::Report`.
- **Invariants:** `ReportBundle` contains no wall-clock timestamp; identical repository state and configuration produces byte-for-byte identical JSON output; bundles architecture circular dependencies and high-centrality modules alongside inventory, findings, and health score.

## Issue #58 — `forge-core`, `forge-git`, `forge-cli` JsonSchema Export (`eforge schema`) (COMPLETED)
- **Crates:** `Cargo.toml`, `crates/forge-core`, `crates/forge-git`, `crates/forge-cli`
- **Public Types & Functions:** `JsonSchema` derived on all types reachable from `ReportBundle` (`Severity`, `Confidence`, `SourceLocation`, `Finding`, `FileNode`, `Module`, `Dependency`, `DimensionScore`, `HealthScore`, `Repository`, `RiskLevel`, `RiskReport`, `FileChange`, `HotspotEntry`, `ChangeImpactReport`, `GitRevision`, `ArchitectureSummary`, `ReportBundle`), `render_report_schema() -> Result<String, ForgeError>`, `run_schema(output: Option<&Path>) -> Result<String, ForgeError>`, `Commands::Schema`.
- **Invariants:** Rust types are the single source of truth for the JSON schema; `render_report_schema()` is deterministic and powers schema validation and TypeScript type generation in `@elcode-forge/schema`.

## Issue #59 — `forge-schema` Workspace Bootstrap, Types & CI (COMPLETED)
- **Packages & Workflows:** `package.json`, `pnpm-workspace.yaml`, `.npmrc`, `packages/forge-schema`, `.github/workflows/ci-web.yml`
- **Public TypeScript Exports (`@elcode-forge/schema`):** `SUPPORTED_SCHEMA_VERSION = 1`, `parseReportBundle(input: unknown) -> ParseReportBundleResult`, and generated types (`ReportBundle`, `Repository`, `Finding`, `HealthScore`, `Module`, etc.).
- **Invariants:** `parseReportBundle` validates untrusted JSON input against `report-bundle.schema.json` using Ajv and verifies `schema_version == 1`; `ci-web.yml` runs path-filtered CI and enforces zero-drift between `eforge schema` and the checked-in JSON Schema.

## Issue #60 — `forge-cli` Read-Only Local `eforge serve` HTTP API (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/serve/{mod.rs, routes.rs}`, `src/cli.rs`, `src/lib.rs`, `Cargo.toml`)
- **Public Types & Functions (`forge_cli::serve`):** `ServeOptions`, `ApiResponse`, `ServeState`, `route_request(state: &ServeState, method: &str, url: &str) -> ApiResponse`, `run_serve(opts: &ServeOptions) -> Result<(), ForgeError>`, `Commands::Serve`.
- **Invariants:** Binds loopback `127.0.0.1:<port>` strictly; only `GET` and `OPTIONS` supported (others return 405); CORS wildcard forbidden (only explicit origin echoed); headers `X-Content-Type-Options: nosniff` and `Cache-Control: no-store` enforced; static file paths validated via `resolve_safe_path`.

## Issue #61 — `forge-cli` History Snapshots & Trends API (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/commands/history.rs`, `src/commands/report.rs`, `src/serve/routes.rs`, `src/cli.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_cli::commands::history`):** `HistorySnapshot`, `HISTORY_DIR` (`"history"`), `save_history_snapshot(repo_root: &Path, bundle: &ReportBundle) -> Result<Option<PathBuf>, ForgeError>`, `load_history(repo_root: &Path, limit: usize) -> Result<Vec<HistorySnapshot>, ForgeError>`, `--save-history` CLI flag, `/api/v1/history?limit=` endpoint.
- **Invariants:** Snapshot files stored as `.eforge/history/<commit_unix_ts>-<sha12>.json`; strictly idempotent per commit; skipped when no git revision is present; `load_history` sorts ascending by commit timestamp and ignores corrupted/malformed files.





