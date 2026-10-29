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

## Issue #62 — `forge-ui` ELCODE Design Tokens & Svelte 5 Core Primitives (COMPLETED)
- **Package:** `packages/forge-ui` (`src/tokens.css`, `src/lib/{ScoreRing,SeverityBadge,DataTable,Card,EmptyState,KeyValue}.svelte`, `src/index.ts`)
- **Public Components & Types (`@elcode-forge/ui`):** `ScoreRing`, `SeverityBadge`, `DataTable`, `Column`, `Card`, `EmptyState`, `KeyValue`.
- **Invariants:** Built on Svelte 5 runes (`$props()`, `$derived()`); strictly avoids `{@html}`; zero external CDN or runtime font downloads; fully accessible with WCAG AA contrast and explicit `aria-*` tags.

## Issue #63 — `dashboard` SvelteKit SPA Scaffold, Dual Data Source & Overview (COMPLETED)
- **App:** `apps/dashboard` (`src/app.html`, `src/lib/data/{source.ts, store.svelte.ts}`, `src/routes/{+layout.svelte, +layout.ts, +page.svelte}`)
- **Public Types & Functions:** `DataSource`, `LoadResult`, `loadFromFile(file: File) -> Promise<LoadResult>`, `loadFromApi(baseUrl: string) -> Promise<LoadResult>`, `dashboardState`.
- **Invariants:** Static SPA compiled via `@sveltejs/adapter-static` with fallback `index.html`; validates all bundles through `parseReportBundle` before rendering; restricts live API URLs strictly to loopback (`127.0.0.1`, `localhost`, `::1`); enforces strict Content-Security-Policy; zero `{@html}`.

## Issue #64 — `dashboard` Interactive Architecture Graph with Cycle Highlighting (COMPLETED)
- **App:** `apps/dashboard` (`src/lib/graph/{toElements.ts, cytoscape.ts}`, `src/routes/architecture/+page.svelte`, `tests/graph.test.ts`, `package.json`)
- **Public Types & Functions:** `GraphNodeData`, `GraphNode`, `GraphEdgeData`, `GraphEdge`, `GraphElements`, `toElements(bundle: ReportBundle): GraphElements`, `initCytoscape(options: CytoscapeOptions): Core`.
- **Invariants:** Pure `toElements` marks cyclic dependency edges (`inCycle: true`) and nodes (`hasCycle: true`) from `bundle.architecture.circular_dependencies`; Dagre layout with technical dark theme; includes keyboard-accessible module table as a non-visual alternative (WCAG AA); detail panel provides parity with `eforge explain` (files, inbound/outbound imports, related findings); zero `{@html}`.

## Issue #65 — `dashboard` Findings, Security, and Dependencies Screens (COMPLETED)
- **App:** `apps/dashboard` (`src/lib/filters.ts`, `src/routes/{findings,security,dependencies}/+page.svelte`, `tests/filters.test.ts`, `src/routes/+layout.svelte`)
- **Public Types & Functions:** `FindingFilter`, `parseFilter(params: URLSearchParams): FindingFilter`, `serializeFilter(filter: FindingFilter): URLSearchParams`, `applyFilter(findings: Finding[], filter: FindingFilter): Finding[]`.
- **Invariants:** Stable ordering preserved in `applyFilter`; filters synchronized with URL search params; evidence is strictly plain-text in untrusted hostiles envelope (zero `{@html}`); dependencies screen flags unpinned/unused/duplicate packages from `deps/*` findings.

## Issue #66 — `dashboard` Changes & Trends Screens (COMPLETED)
- **App:** `apps/dashboard` (`src/lib/data/history.ts`, `src/lib/charts/Sparkline.svelte`, `src/routes/{changes,trends}/+page.svelte`, `tests/history.test.ts`, `src/routes/+layout.svelte`)
- **Public Types & Functions:** `HistorySnapshot`, `HistoryApiResponse`, `loadHistory(baseUrl: string, limit?: number): Promise<HistorySnapshot[]>`, `Sparkline`.
- **Invariants:** `loadHistory` strictly enforces local loopback URLs; Trends screen provides clear fallback for static file mode explaining `eforge report --save-history` + `eforge serve` requirement; Sparkline rendered as accessible, dependency-free SVG polylines; Changes screen surfaces blast radius breakdown, missing test warnings, and Git churn hotspots.

## Issue #67 — `dashboard` E2E Smoke, Dogfood Integration & Phase 3 Docs (COMPLETED)
- **App & CI:** `apps/dashboard` (`playwright.config.ts`, `serve-preview.js`, `e2e/screens.spec.ts`, `e2e/fixtures/next-app.bundle.json`, `package.json`), `.github/workflows/eforge-dogfood.yml`, `README.md`.
- **Public Capabilities:** End-to-end smoke testing across all 7 dashboard screens (Overview, Architecture, Findings, Security, Dependencies, Changes, Trends); Dogfood GitHub Action generates deterministic report bundle, builds dashboard SPA, and uploads `forge-report.json` artifact; full user-facing dashboard documentation in README.md.
- **Milestone Status:** Closes EPIC #55 and completes **Phase 3 (Report Bundle, Local Daemon & Web Intelligence Dashboard)**.

## Issue #85 — `forge-core` AI Configuration Section, Untrusted Boundary, and ForgeError::Ai (COMPLETED)
- **Crate:** `crates/forge-core` (`src/config.rs`, `src/untrusted.rs`, `src/error.rs`, `src/lib.rs`) and `crates/forge-mcp` (`src/boundary.rs`)
- **Public Types & Functions (`forge_core::{config, untrusted, error}`):** `AiProviderKind { OpenAiCompatible, Anthropic, Gemini }`, `AiConfig`, `ForgeConfig.ai`, `ForgeError::Ai`, `ForgeError::AiBudgetExceeded`, `UntrustedEnvelope<T>`, `detect_prompt_injection_signals`, `wrap_untrusted_payload`, `UNTRUSTED_REPO_DATA_LEVEL`, `UNTRUSTED_SECURITY_NOTICE`.
- **Invariants:** `ai.enabled` defaults to `false`; secret keys (`api_key`, `token`, `secret`, `password`) in `ai:` configuration are strictly rejected with `ForgeError::Config`; credentials must be passed via environment variable names (`api_key_env`); `forge-mcp::boundary` re-exports canonical implementations from `forge_core::untrusted`.

## Issue #86 — `forge-ai` Scaffold, AiProvider Trait, and MockProvider (COMPLETED)
- **Crate:** `crates/forge-ai` (`src/{lib.rs, types.rs, provider.rs, mock.rs}`) + Workspace (`Cargo.toml`, `AGENTS.md`)
- **Public Types & Functions (`forge_ai`):** `AiProvider` trait (`name`, `generate`, `stream`, `embed`, `structured_output`), `AiRole { System, User, Assistant }`, `AiMessage`, `AiRequest`, `AiResponse`, `TokenUsage`, `ensure_ai_enabled(&AiConfig) -> Result<(), ForgeError>`, `MockProvider` (scripted responses, usage tracking, request recorder).
- **Invariants:** `forge-ai` depends strictly on `forge-core`, `serde`, `serde_json`, `ureq`, and `blake3`; zero-panic policy strictly enforced; offline execution supported via `MockProvider`.

## Issue #87 — `forge-ai` OpenAI-Compatible Provider (COMPLETED)
- **Crate:** `crates/forge-ai` (`src/providers/{mod.rs, http.rs, openai_compat.rs}`)
- **Public Types & Functions (`forge_ai`):** `OpenAiCompatProvider::new(base_url, api_key, model, timeout_secs) -> Result<Self, ForgeError>`, `AiProvider` implementation (`generate`, `stream` via SSE, `embed`, `structured_output`), `create_agent`, `read_bounded_body`, `map_ureq_error`, `MAX_RESPONSE_BYTES` (4 MiB).
- **Invariants:** Compatible with OpenAI, OpenRouter, Ollama, LM Studio, vLLM; optional `api_key` for local models; zero leaking of raw API keys in HTTP error diagnostics; bounded 4 MiB response reads prevent memory exhaustion.

## Issue #88 — `forge-ai` Anthropic, Gemini, and Provider Factory (COMPLETED)
- **Crate:** `crates/forge-ai` (`src/providers/{anthropic.rs, gemini.rs}`, `src/factory.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_ai`):** `AnthropicProvider`, `GeminiProvider`, `build_provider(cfg: &AiConfig, env_lookup: &dyn Fn(&str) -> Option<String>) -> Result<Box<dyn AiProvider>, ForgeError>`, `mask_secret(raw: &str) -> String`.
- **Invariants:** Credentials resolved exclusively via `env_lookup(api_key_env)` or standard vendor fallback env vars; `mask_secret` safely redacts keys for display; `AnthropicProvider::embed` returns explicit `ForgeError::Ai` unsupported error; closes Epic #82 and completes **Phase 4A (AI Foundation & Provider Abstraction)**.

## Issue #89 — `forge-ai` Deterministic Context Retrieval Builder (COMPLETED)
- **Crate:** `crates/forge-ai` (`src/{tokens.rs, prompt.rs, context.rs, lib.rs}`)
- **Public Types & Functions (`forge_ai`):** `estimate_tokens(text: &str) -> u32`, `AiTask { ExplainModule, ReviewDiff, GeneralQuery }`, `system_prompt_for_task(task: AiTask) -> &'static str`, `SectionKind`, `ContextSection`, `CodeSnippet`, `ContextInput`, `ContextPack`, `build_context(input: &ContextInput, max_context_tokens: u32) -> Result<ContextPack, ForgeError>`, `sections_from_repository(repo: &Repository, focus: Option<&str>) -> Vec<ContextSection>`.
- **Invariants:** Identical `ContextInput` + token budget always produces byte-for-byte identical `ContextPack`; candidate sections are prioritized deterministically (Security Findings > Architecture > Dependencies > Code Snippets); user payload is safely wrapped in `UntrustedEnvelope` and scanned via `detect_prompt_injection_signals`.

## Issue #90 — `forge-ai` Cost Controls, Response Cache, and Usage Ledger (COMPLETED)
- **Crate:** `crates/forge-ai` (`src/{budget.rs, cache.rs, usage.rs, lib.rs}`)
- **Public Types & Functions (`forge_ai`):** `preflight_check(pack: &ContextPack, cfg: &AiConfig) -> Result<(), ForgeError>`, `compute_cost_usd(input_tokens: u32, output_tokens: u32, cfg: &AiConfig) -> Option<f64>`, `enforce_daily_budget(summary: &UsageSummary, cfg: &AiConfig) -> Result<(), ForgeError>`, `ResponseCache` (`new`, `key`, `get`, `put`), `UsageRecord`, `UsageSummary`, `append_usage(repo_root: &Path, rec: &UsageRecord) -> Result<(), ForgeError>`, `summarize_usage(repo_root: &Path, today: &str) -> Result<UsageSummary, ForgeError>`.
- **Invariants:** Cache stores JSON responses under `<repo_root>/.eforge/ai-cache/<key>.json` keyed by deterministic 64-char BLAKE3 hash; strictly validates hex keys to eliminate path traversal; response cache never stores headers or auth secrets; append-only ledger at `<repo_root>/.eforge/ai-usage.jsonl` safely ignores corrupted lines without panicking; closes Epic #83 and completes **Phase 4B (Deterministic Context Retrieval & Cost Controls)**.

## Issue #91 — `forge-cli` AI-Augmented Explain Command with Dry-Run and Grounding (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/ai/{mod.rs, orchestrate.rs}`, `src/commands/explain.rs`, `src/cli.rs`, `src/lib.rs`) and `crates/forge-security` (`src/secrets.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_cli` / `forge_security`):** `redact_secrets(content: &str) -> String`, `GroundedClaim`, `AiExplainAnswer`, `AiExplainOutput`, `AiExplainOptions`, `execute_explain_with_ai`, `execute_explain_ai`.
- **Invariants:** Deterministic facts are always computed and rendered first; sensitive files (`.env*`, `*.pem`, `*.key`, `.git/**`, `.eforge/**`) are excluded before snippet collection; all code snippets are redacted via `redact_secrets` before entering context; `--dry-run` outputs exact `ContextPack` with token estimates without network or provider instantiation; AI risks verify cited sources against repository modules, files, and findings (`[verified]` vs `[unverified]`).

## Issue #92 — `forge-cli` eforge ai-review with Grounding Guard (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/commands/ai_review.rs`, `src/commands/mod.rs`, `src/cli.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_cli`):** `AiClassification { Confirmed, Probable, Suggestion }`, `AiReviewItem`, `AiReviewReport`, `AiReviewOptions`, `apply_deterministic_guard`, `execute_ai_review`, `render_ai_review_report`.
- **Invariants:** Deterministic findings and change risk in `ReviewReport` are rendered verbatim first; `apply_deterministic_guard` enforces that unlinked `Confirmed` items are downgraded to `Probable`, and items referencing hallucinated/unknown files are discarded; exit code evaluation for `--fail-on` is strictly bound to deterministic findings; AI outputs cannot suppress or alter exit codes.

## Issue #93 — `forge-cli` eforge ai status and usage commands (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/commands/ai_admin.rs`, `src/commands/mod.rs`, `src/cli.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_cli`):** `AiStatusReport`, `AiUsageReport`, `run_ai_status(repo_root: &Path, ping: bool) -> Result<AiStatusReport, ForgeError>`, `run_ai_usage(repo_root: &Path) -> Result<AiUsageReport, ForgeError>`, `render_ai_status_report`, `render_ai_usage_report`.
- **Invariants:** `eforge ai status` without `--ping` makes zero network calls; API credentials are never output in raw form and are strictly masked via `mask_secret`; `eforge ai usage` summarizes `.eforge/ai-usage.jsonl` safely and tracks remaining daily budget against `budget_daily_usd`.

## Issue #94 — AI Quality Evaluation Suite, GitHub Action, Privacy Docs & v0.4.0 (COMPLETED)
- **Crates & Files:** `fixtures/ai-eval/questions.json`, `crates/forge-cli/tests/ai_eval.rs`, `integrations/github-action/{action.yml, README.md}`, `docs/AI_AND_PRIVACY.md`, `eforge.config.yaml`, `README.md`.
- **Public Capabilities & Invariants:**
  - `fixtures/ai-eval/questions.json`: Golden benchmark dataset covering `next-app`, `ts-circular`, and `secret-leak` with deterministic grounding facts and anti-hallucination assertions.
  - `crates/forge-cli/tests/ai_eval.rs`: Comprehensive test suite for retrieval recall, secret redaction before prompt building, deterministic grounding guards (downgrading ungrounded `Confirmed` items, dropping hallucinated files), and report byte determinism.
  - `integrations/github-action`: Added opt-in `ai-review: 'false'` input (defaulting strictly to false), allowing automated AI PR reviews when API keys are passed via GitHub secrets.
  - `docs/AI_AND_PRIVACY.md`: Comprehensive privacy architecture guide detailing zero data exfiltration with local providers (Ollama, LM Studio), prompt injection defense envelopes, in-flight secret redaction, and token budgeting.
  - **Milestone Status:** Closes EPIC #84 and marks **Phase 4 (AI Engine & Codebase Intelligence)** fully completed.

## ELCODE Studio Manifesto (`01`–`05`) & Green-CI Workflow Discipline (COMPLETED)
- **Files:** `.github/workflows/{ci-rust.yml, ci-web.yml, eforge-dogfood.yml, security-durability.yml, release-binaries.yml}`, `README.md`, `AGENTS.md`.
- **Invariants:**
  - All 5 ELCODE Studio Manifesto principles (`https://elcode.id/`: `01` Instrument not compromise, `02` Code quality visible in behavior/speed/durability, `03` AI as leverage not identity, `04` Precision over decoration, `05` Build things worth keeping) are embedded in `README.md` and `AGENTS.md`.
  - Node/pnpm workflows (`ci-web.yml`, `eforge-dogfood.yml`) use deterministic `npm install -g pnpm@12.8.1` + `actions/cache@v4` on `pnpm store path --silent` to avoid `pnpm/action-setup@v4` self-installer incompatibilities.
  - `security-durability.yml` enforces `cargo audit` (RustSec Advisory DB) and `eforge security --format sarif`; `release-binaries.yml` builds cross-platform `--locked` binaries with `SHA256SUMS.txt`.
  - **Mandatory Green-CI Pre-Merge Gate:** Every PR must pass `gh pr checks <PR_NUMBER> --watch` 100% green before running `gh pr merge --squash`.

## Issue #110 — Declarative Custom Rule Schema, IR Models, and Rule Loader (COMPLETED)
- **Crates:** `crates/forge-rules` (`src/declarative.rs`, `src/lib.rs`) and `crates/forge-core` (`src/config.rs`)
- **Public Types & Functions (`forge_rules` / `forge_core`):**
  - `RuleCategory { Architecture, Quality, Security, Dependencies }` with `as_str`, `Display`, `FromStr`.
  - `RuleSource { Builtin(String), Pack(String), Custom(String) }`.
  - `MatcherKind { ForbiddenImports, RequiredCompanion, FileNaming, FileSize, RegexPattern, DependencyRestriction, Unknown }`.
  - `RuleMatcher`, `DeclarativeRule`, `RuleFile`.
  - `validate_rule_id(id: &str) -> Result<(), ForgeError>`.
  - `validate_rule(rule: &DeclarativeRule) -> Result<(), ForgeError>`.
  - `parse_rule_file(content: &str, source: RuleSource) -> Result<Vec<DeclarativeRule>, ForgeError>`.
  - `load_declarative_rules(repo_root: &Path, custom_paths: &[String]) -> Result<Vec<DeclarativeRule>, ForgeError>`.
  - `ForgeConfig`: added `rule_packs: Vec<String>` and `custom_rule_paths: Vec<String>` with full `eforge.config.yaml` parsing.
- **Invariants:**
  - Rule IDs must strictly follow `<category>/<slug>` (`arch`, `quality`, `security`, `deps`).
  - Safe path resolution via `path_guard::resolve_safe_path` enforces that all rule files and packs stay strictly within the analyzed repository root, preventing path traversal attacks (`../`).
  - Zero-panic parsing of both YAML and JSON rule specifications with line-aware diagnostics.

## Issue #111 — Six Deterministic Rule Matchers & Engine Integration (COMPLETED)
- **Crate:** `crates/forge-rules` (`src/matchers/`, `src/engine.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_rules::matchers` & `forge_rules::engine`):**
  - Six deterministic matchers:
    - `forbidden_imports`: enforces module import boundaries against forbidden target patterns.
    - `required_companion`: verifies source files have corresponding test companion files.
    - `file_naming`: validates file naming conventions (`kebab-case`, `snake_case`, `camelCase`, `PascalCase`, wildcard patterns).
    - `file_size`: enforces maximum line count (`max_lines`) and byte limits (`max_bytes`).
    - `regex_pattern`: scans source text for forbidden tokens or antipatterns with safe path guard and per-file caps.
    - `dependency_restriction`: detects prohibited packages and unpinned wildcard versions (`*`, `latest`).
  - `evaluate_declarative_rule(rule: &DeclarativeRule, repo: &Repository) -> Result<Vec<Finding>, ForgeError>`.
  - `RuleEngine`: added `declarative_rules: Vec<DeclarativeRule>`, `with_declarative_rules`, `add_declarative_rule`, and `declarative_rules()`.
- **Invariants:**
  - Evaluates both built-in and declarative rules within `RuleEngine::evaluate`.
  - Deterministic findings sorting by `(Severity::desc, id::asc)` and deduplication.
  - Zero-panic execution across all matchers with canonical path-guard protection for file-reading matchers.

## Issue #113 — Rule Packs Engine & Config Extends Hierarchy (COMPLETED)
- **Crates:** `crates/forge-rules` (`src/packs.rs`, `src/engine.rs`, `src/lib.rs`) and `crates/forge-core` (`src/config.rs`)
- **Public Types & Functions (`forge_rules::packs` & `forge_rules::engine`):**
  - `RulePack`: pre-packaged architectural, quality, or security rule bundle.
  - Four built-in packs:
    - `recommended`: standard engineering baseline (`no-direct-db-in-ui`, `no-circular-dependencies`, `require-service-tests`, `max-file-lines: 300`).
    - `strict`: elevated discipline (critical architecture boundaries, 250-line file limit, strict test companions, unpinned version prevention).
    - `security`: code hardening (prohibits `eval()`, `child_process.exec()`, debugger statements, `dangerouslySetInnerHTML`, and known vulnerable packages).
    - `clean-architecture`: hexagonal/onion layer boundaries (`ui-no-infra`, `domain-pure`, `infra-no-ui`, `ui-no-db`).
  - `get_builtin_pack(name: &str) -> Option<RulePack>` and `list_builtin_packs() -> Vec<RulePack>`.
  - `resolve_rule_hierarchy(repo_root: &Path, config: &ForgeConfig) -> Result<(Vec<RuleConfig>, Vec<DeclarativeRule>), ForgeError>`.
  - `RuleEngine::from_config(repo_root: &Path, config: &ForgeConfig) -> Result<RuleEngine, ForgeError>`.
  - `ForgeConfig`: added inline array parsing support (`extends: ["recommended", "clean-architecture"]`).
- **Invariants:**
  - Inheritance precedence strictly enforced: `Built-in Defaults < Extended Packs < Custom Local Rules < Per-rule Overrides`.
  - Projects without explicit extends implicitly extend `recommended` for backward compatibility.

## Issue #114 — CLI Rules Command (List, Check, Init) & Scanner Integration (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/commands/rules.rs`, `src/cli.rs`, `src/commands/scan.rs`, `src/commands/doctor.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_cli::commands::rules`):**
  - CLI Subcommands:
    - `eforge rules [list]`: renders rich rule catalog with ID, category, severity, source (`builtin`, `pack:*`, `custom:*`), and status; supports `--category` and `--enabled-only` filters.
    - `eforge rules check [path]`: verifies rule syntax without scanning; validates against schema with zero panics.
    - `eforge rules init`: creates `.eforge/rules/custom-rules.yaml` with starter examples for all 6 matchers.
  - Reports: `RulesListReport`, `RulesCheckReport`, `RulesInitReport`, and `RuleItemDescriptor`.
  - Formatters: terminal table, JSON (`--format json`), and Markdown (`--format markdown`).
  - `eforge scan`: updated to instantiate `RuleEngine::from_config(repo_root, config)` to evaluate declarative rules alongside built-in rules.
  - `eforge doctor`: added Check 6 for architectural & declarative rules hygiene.
- **Invariants:**
  - `eforge rules check` returns `ExitCode::ConfigurationError` on invalid syntax.
  - Starter template includes valid examples for all 6 matchers (`forbidden_imports`, `required_companion`, `file_naming`, `file_size`, `regex_pattern`, `dependency_restriction`).


## Issue #115 — `forge-mcp` `eforge_get_rules` MCP Tool with Untrusted Envelope (COMPLETED)
- **Crate:** `crates/forge-mcp` (`src/tools.rs`, `src/server.rs`, `src/lib.rs`)
- **Public Types & Functions (`forge_mcp::tools` & `forge_mcp::server`):**
  - Tool Name: `eforge_get_rules` added to `list_mcp_tools()` (catalog expanded from 6 to 7 tools).
  - Descriptor Struct: `McpRuleDescriptor` with `id`, `name`, `category`, `severity`, `description`, `remediation`, `source`, and `enabled`.
  - Input Schema: `{ "category"?: string, "enabled_only"?: boolean }`.
  - Tool Handler: `call_mcp_tool(..., "eforge_get_rules", arguments)` resolves active rule hierarchy via `resolve_rule_hierarchy`, filters by category/enabled status, sorts deterministically by `(Severity::desc, id::asc)`, and wraps payload in `UntrustedEnvelope` with prompt-injection defense signal detection.
- **Invariants:**
  - Fast execution: `eforge_get_rules` does not run full AST parsing, resolving rule configurations directly via `load_config` and `resolve_rule_hierarchy`.
  - Strict containment: rule descriptors and filtering arguments are sanitized and protected within `UntrustedEnvelope` with `trust_level: "untrusted_repository_data"`.


## Issue #117 — ReportBundle Rules Schema Evolution & Rules Catalog Screen (COMPLETED)
- **Crate & Packages:** `crates/forge-cli` (`src/commands/report.rs`, `src/commands/schema.rs`), `packages/forge-schema`, `apps/dashboard`
- **Public Types & Functions:**
  - Rust: `RuleDescriptor` struct deriving `JsonSchema` added to `ReportBundle::rules` in `crates/forge-cli/src/commands/report.rs`.
  - TypeScript (`@elcode-forge/schema`): `RuleDescriptor` exported from `@elcode-forge/schema` and embedded into `ReportBundle`.
  - Dashboard: `/rules` route (`apps/dashboard/src/routes/rules/+page.svelte`) with category tabs, source pack selector, search filter, enabled toggle, and project violation count counters.
  - Deep linking: Findings rule IDs link to `/rules?id=...`, and rule violation cards link to `/findings?rule_prefix=...`.
- **Invariants:**
  - `build_report_bundle` resolves active rule hierarchy via `RuleEngine::from_config` and sorts rules deterministically by `(Severity::desc, id::asc)`.
  - Zero schema drift between `ReportBundle` Rust IR and `report-bundle.schema.json`.


## Issue #118 — Custom Rules Fixture, E2E Evaluation Suite, Rules Guide & v0.5.0 Release (COMPLETED)
- **Crates, Packages & Fixtures:** `fixtures/custom-rules-app`, `crates/forge-cli` (`tests/custom_rules_e2e.rs`), `docs/RULES_GUIDE.md`, `README.md`, `Cargo.toml`, `package.json`
- **Capabilities & Deliverables:**
  - `fixtures/custom-rules-app`: Curated fixture demonstrating all 6 custom matchers (`forbidden_imports`, `required_companion`, `file_naming`, `file_size`, `regex_pattern`, `dependency_restriction`) and compliant files with zero false positives.
  - `tests/custom_rules_e2e.rs`: Integration suite verifying `eforge rules list`, `eforge rules check`, `eforge scan`, and `eforge report` against `fixtures/custom-rules-app`.
  - `docs/RULES_GUIDE.md`: Living guide detailing custom rule syntax, matcher schemas, pack extension precedence, and MCP tool usage.
  - Version Release: Bumped workspace to `v0.5.0` across Rust crates (`Cargo.toml`) and TypeScript packages (`packages/forge-schema`, `packages/forge-ui`, `apps/dashboard`).
- **Invariants:**
  - Zero-panic test discipline: integration test suite complies strictly with zero `.unwrap()` and `.expect()` policy.
  - Zero schema drift and 100% green CI verification across all dogfooding and release workflows.


## Issue #130 — Native Git Hooks Lifecycle & Configuration Schema (COMPLETED)
- **Crates:** `crates/forge-core` (`src/config.rs`), `crates/forge-git` (`src/hooks.rs`, `src/lib.rs`)
- **Public Types & Functions:**
  - `forge_core::config`: `HooksConfig`, `HookActionConfig` added to `ForgeConfig::hooks`.
  - `forge_git::hooks`: `HookType` (`PreCommit`, `PrePush`, `All`), `HookEntryStatus`, `HookStatusReport`, `HookInstallResult`, `HookUninstallResult`, `install_hooks(repo_root, hook_type, force)`, `uninstall_hooks(repo_root, hook_type)`, `get_hooks_status(repo_root)`.
- **Invariants:**
  - Non-destructive backup preservation: foreign pre-existing hooks are copied to `<hook>.forge-backup` and restored on uninstall.
  - Executable permissions: hook scripts are marked `0o755` on Unix.
  - Bypass support: generated shell scripts check `FORGE_SKIP_HOOK=1` before attempting execution.
  - Worktree/submodule awareness: resolves `.git` when it is a file (`gitdir: <path>`).

## Issue #131 — `eforge hook` Command & Sub-200ms Fast Staged-Diff Scanner (COMPLETED)
- **Crates:** `crates/forge-cli` (`src/commands/hook.rs`, `src/commands/mod.rs`, `src/cli.rs`, `src/lib.rs`)
- **CLI Commands & Structs:**
  - `Commands::Hook { action: HookAction }`
  - `HookAction::Install { hook_type, force }`
  - `HookAction::Uninstall { hook_type }`
  - `HookAction::Status { format }`
  - `HookAction::Run { hook_type, staged_only, format }`
  - `HookRunOutput`, `HookRunResult` with formatted output in terminal, JSON, and Markdown.
- **Capabilities & Performance:**
  - Fast staged files collector: `git diff --cached --name-only --diff-filter=ACMR` with `< 10ms` fast-path when no staged files exist.
  - Sub-200ms staged-diff evaluation: `execute_review` on cached diff with blast-radius, change impact, and rule checking scoped strictly to staged changes.
  - Pre-push hook verification: runs fast review over unpushed commits (`origin/HEAD..HEAD`).
  - Formatter support: table/plain terminal UX, machine-readable JSON, and markdown output.
- **Invariants:**
  - Non-zero exit code (`1` or `4`) if any violations exceed configured `fail_on` severity threshold, blocking `git commit` or `git push`.
  - Zero-panic test discipline across all CLI command flows.

## Issue #133 — TTY-Aware ANSI Theme, NO_COLOR Compliance & Visual Health Meter (COMPLETED)
- **Crates:** `crates/forge-cli` (`src/theme.rs`, `src/lib.rs`)
- **Public Types & Functions:**
  - `ColorMode { Auto, Always, Never }`
  - `Theme { color_enabled: bool, unicode_enabled: bool }`
  - `is_color_supported(mode: ColorMode) -> bool`, `is_unicode_supported(mode: ColorMode) -> bool`
  - Theme styling primitives: `bold`, `dim`, `cyan`, `green`, `yellow`, `red`, `magenta`, `white`, `blue`, `gray`
  - Severity badge formatters: `badge_critical`, `badge_high`, `badge_medium`, `badge_low`, `badge_info`, `badge_severity(Severity)`
  - Status markers: `status_ok`, `status_fail`, `status_warn`
  - Health rating: `health_rating(score: u8) -> &'static str`
  - Score Meter: `render_score_meter(score: u8, color_mode: ColorMode) -> String`
- **Invariants:**
  - Zero external color dependencies; purely native ANSI escape codes.
  - Strict compliance with `NO_COLOR`, `CLICOLOR_FORCE`, and `CLICOLOR` environment standards.
  - Deterministic plain ASCII fallback (`[=========.] 92/100 (Good)`) for non-TTY/piped environments.

## Issue #134 — Rich Terminal Formatters for Scan, Review & Findings (COMPLETED)
- **Crates:** `crates/forge-cli` (`src/formatters/terminal.rs`, `src/commands/doctor.rs`, `src/commands/review.rs`)
- **Public Types & Functions:**
  - `render_finding_card(finding: &Finding, theme: &Theme) -> String`: renders structured finding cards with colored severity badge, bold title/rule ID, clickable `location`, and border-styled snippet evidence blocks.
  - `render_terminal(repo: &Repository) -> String` and `render_terminal_with_theme(repo: &Repository, theme: &Theme) -> String`: renders multi-phase milestone progress checklist (`Languages`, `Dependencies`, `Architecture`, `Rules`), visual score bar meter, and tabular dimension breakdown with Unicode box-drawing or ASCII fallback.
  - `render_doctor_terminal_with_theme(report: &DoctorReport, theme: &Theme) -> String`: modern doctor output with styled status indicators (`[OK]`, `[WARN]`, `[FAIL]`) and elapsed latency metrics.
  - `render_review_report(report: &ReviewReport, format: OutputFormat)`: upgraded review output with colored risk tiers and structured cards.
- **Invariants:**
  - Automatic detection of TTY vs CI/pipe environments ensures clean, zero-escape ASCII formatting when redirected or when `NO_COLOR` is present.
  - Byte-for-byte deterministic output for identical input repositories.

## Issue #136 — Zero-Install npm Package `@elcode-forge/cli` (COMPLETED)
- **Package:** `packages/forge-cli-npm/` (`@elcode-forge/cli`)
- **Public Modules & Executables:**
  - `bin/eforge.js`: zero-install executable entrypoint registered under `bin.eforge` and `bin["elcode-forge"]`.
  - `src/index.js`: zero-dependency native launcher.
  - Functions: `resolveTarget`, `getCacheDir`, `getBinaryPath`, `parseSha256Sums`, `verifyChecksum`, `downloadFile`, `ensureBinary`, `runCli`.
- **Capabilities:**
  - Automated release target resolution: `win32-x64` (`x86_64-pc-windows-msvc`), `linux-x64` (`x86_64-unknown-linux-gnu`), `darwin-arm64` (`aarch64-apple-darwin`).
  - Cache management: caches binary in `~/.eforge/bin/v<version>/` or `%LOCALAPPDATA%\eforge\bin\v<version>\`.
  - Cryptographic verification: downloads and verifies official `SHA256SUMS.txt` before unpacking.
  - Zero external npm runtime dependencies: uses pure Node.js built-ins (`node:https`, `node:crypto`, `node:fs`, `node:child_process`).
  - Environment overrides: `EFORGE_BINARY_PATH`, `EFORGE_CACHE_DIR`, `EFORGE_DOWNLOAD_URL`.
- **Invariants:**
  - Strict SHA256 integrity verification rejects any modified or corrupt binary downloads.
  - Clean exit code and stdio propagation from child process.

## Issue #137 — Standalone Installers (POSIX & Windows), Homebrew Tap & cargo-binstall (COMPLETED)
- **Files & Manifests:** `scripts/install.sh`, `scripts/install.ps1`, `packaging/homebrew/eforge.rb`, `crates/forge-cli/Cargo.toml`
- **Capabilities & Deliverables:**
  - `scripts/install.sh`: POSIX installer detecting OS (`Linux`, `Darwin`) and architecture (`x86_64`, `arm64`), downloading official release archives, verifying `SHA256SUMS.txt`, installing to `~/.local/bin` (or `/usr/local/bin`), and validating execution.
  - `scripts/install.ps1`: Native PowerShell installer with `Get-FileHash` SHA256 integrity check, extraction to `$env:LOCALAPPDATA\eforge\bin\`, and automated User `PATH` environment registration.
  - `packaging/homebrew/eforge.rb`: Homebrew formula supporting macOS (Apple Silicon arm64) and Linux x86_64 with cryptographic checksums.
  - `[package.metadata.binstall]` in `crates/forge-cli/Cargo.toml`: automated 1-command installation via `cargo binstall eforge`.
- **Invariants:**
  - Cryptographic verification mandatory: all installers abort with non-zero exit codes if SHA256 checksum fails.
  - Dual binary support: installs both primary `eforge` and studio alias `elcode-forge`.

## Issue #138 — Git Hooks Integration Tests, Release Prep & Monorepo v0.6.0 Bump (COMPLETED)
- **Crates, Packages & Tests:** `crates/forge-cli/tests/hooks_e2e.rs`, `README.md`, `Cargo.toml`, `package.json`, `packages/forge-schema`, `packages/forge-ui`, `packages/forge-cli-npm`, `apps/dashboard`, `packaging/homebrew/eforge.rb`
- **Capabilities & Deliverables:**
  - `crates/forge-cli/tests/hooks_e2e.rs`: Integration test suite exercising git hooks lifecycle (`install`, `status`, `uninstall`), clean fast path when no staged files exist, and pre-commit detection & rejection of architectural violations in staged files.
  - Distribution documentation in `README.md` covering zero-install (`npx @elcode-forge/cli`), standalone POSIX installer (`scripts/install.sh`), PowerShell installer (`scripts/install.ps1`), Homebrew tap (`packaging/homebrew/eforge.rb`), and `cargo binstall`.
  - CLI command reference expanded to 18 subcommands including `eforge hook`.
  - Synchronized monorepo workspace version to `v0.6.0` across Rust crates and TypeScript packages.
- **Invariants:**
  - Zero-panic test discipline maintained across all integration suites.
  - Zero schema drift between `@elcode-forge/schema` and Rust IR types.
  - Complete Phase 6 Git Hooks, Terminal UX & Zero-Install Distribution milestone achieved.


## Release v0.6.0 — Git Hooks, Terminal UX Modernization & Zero-Install Distribution (OFFICIAL RELEASE)
- **Tag:** `v0.6.0` (PR #146 merged into `main` and fast-forwarded to `develop`)
- **Published Release Assets:**
  - `eforge-v0.6.0-aarch64-apple-darwin.tar.gz`
  - `eforge-v0.6.0-x86_64-pc-windows-msvc.zip`
  - `eforge-v0.6.0-x86_64-unknown-linux-gnu.tar.gz`
  - `SHA256SUMS.txt`
- **Release Highlights:** Native Git Hooks Engine (`eforge hook`), TTY-aware ANSI Terminal UX & Health Score Meter, Zero-Install npm distribution (`npx @elcode-forge/cli`), Standalone POSIX & Windows Installers, and Homebrew formula.
- **Invariants:** 100% green CI gate across all workflows; zero open issues in milestone.


## Zero-Install Distribution Package Alignment (`elcode-forge`)
- **Package Name:** `elcode-forge` (Unscoped package for friction-free `npx elcode-forge doctor` and `npm install -g elcode-forge`).
- **Release Automation:** Added `.github/workflows/publish-npm.yml` supporting automated publishing to `registry.npmjs.org` upon release publication and manual `workflow_dispatch`.
- **Homebrew Formula (`packaging/homebrew/eforge.rb`):** Synchronized exact cryptographic SHA-256 hashes matching official GitHub Release `v0.6.0` (`aarch64-apple-darwin` and `x86_64-unknown-linux-gnu`).
- **Installers (`scripts/install.sh` & `scripts/install.ps1`):** Pointed installation references to `main` branch with `v0.6.0` fallback.

- **Homebrew Tap Repository:** Created and initialized `github.com/elcode-id/homebrew-tap` with `Formula/eforge.rb` enabling immediate `brew install elcode-id/tap/eforge`.


## Issue #150 — Python Manifest & Lockfile Dependency Parsers (COMPLETED)
- **Crate:** `crates/forge-analyzer` (`src/python_manifest.rs`, `tests/python_manifest_test.rs`)
- **Public Types & Functions:**
  - `ParsedPythonManifest`: struct holding `dependencies: Vec<Dependency>`, `duplicate_packages: Vec<String>`, `unpinned_packages: Vec<String>`.
  - `normalize_python_package_name(raw: &str) -> String`: PEP 503 normalization (lowercased, extras stripped, underscores to hyphens).
  - `is_python_unpinned(version_req: &str) -> bool`: flags missing versions, wildcards, or unbounded upper-only limits.
  - `parse_requirement_line(line: &str) -> Option<(String, String)>`: parses requirements with comments, directives, and environment markers stripped.
  - `parse_pyproject_toml_content(content: &str, lock_versions: &BTreeMap<String, String>) -> Result<ParsedPythonManifest, ForgeError>`: supports both PEP 621 (`[project.dependencies]`, `[project.optional-dependencies]`) and Poetry (`[tool.poetry.dependencies]`, `[tool.poetry.group.*.dependencies]`).
  - `parse_requirements_txt_content(content: &str, lock_versions: &BTreeMap<String, String>) -> Result<ParsedPythonManifest, ForgeError>`.
  - `read_python_lockfile_versions(repo_root: &Path) -> BTreeMap<String, String>`: resolves exact versions from `poetry.lock`, `uv.lock`, and `Pipfile.lock`.
  - `parse_python_manifests(repo_root: &Path) -> Result<ParsedPythonManifest, ForgeError>`.
- **Invariants:**
  - Zero-panic: all parsing operations map errors to `ForgeError::Serialization` or `ForgeError::Io`.
  - Dependencies are sorted deterministically by `(name, is_dev)` with `ecosystem: "pypi"`.


## Issue #151 — Python Import Graph Extractor, Module Resolver & Cycle Detector (COMPLETED)
- **Crate:** `crates/forge-analyzer` (`src/python_imports.rs`, `src/graph.rs`, `src/lib.rs`, `tests/python_imports_test.rs`)
- **Public Types & Functions:**
  - `PYTHON_STDLIB_MODULES: &[&str]`: Immutable catalog of 200+ Python standard library modules across Python 3.8–3.13.
  - `is_python_stdlib(module_name: &str) -> bool`: Filters standard library imports vs local internal imports vs third-party PyPI packages.
  - `strip_python_comments(source: &str) -> String`: Strips `#` line comments and `"""` / `'''` docstrings while preserving line breaks.
  - `extract_python_imports_and_exports(source: &str) -> (Vec<String>, Vec<String>)`: Extracts `import`, `from ... import`, relative imports, multiline parenthesized imports, and top-level exports (`def`, `async def`, `class`, `__all__`).
  - `resolve_python_import(importer_rel_path: &str, raw_import: &str, known_files: &BTreeSet<String>) -> Option<String>`: Resolves relative (`.`, `..`, `...`) and package imports (`app/...`, `src/...`) to internal repository `.py` files.
  - `extract_pypi_package_name(raw_import: &str) -> Option<String>`: Extracts normalized PyPI package name for third-party imports.
  - `build_python_architecture_graph(repo_root: &Path, files: &[FileNode]) -> Result<ArchitectureGraph, ForgeError>`: Infers logical module boundaries (`app/routers`, `app/services`, etc.), PyPI external dependencies, high-centrality modules, and Tarjan SCC circular import cycles.
- **Invariants:**
  - Zero-panic: all parsing and graph construction operations strictly propagate errors or handle empty/malformed lines safely.
  - Standard library imports are filtered out from `external_imports` to avoid false-positive external dependencies.


## Issue #152 — Python Unsafe Pattern Rules, Framework Detectors & fixtures/fastapi-app (COMPLETED)
- **Crates:** `crates/forge-security`, `crates/forge-analyzer`, `crates/forge-cli`, `fixtures/fastapi-app/`
- **Public Types & Functions:**
  - `RULE_PYTHON_UNSAFE_EVAL: &str = "security/python-unsafe-eval"`: Flags dynamic evaluation via `eval(...)` and `exec(...)`.
  - `RULE_PYTHON_INSECURE_SUBPROCESS: &str = "security/python-insecure-subprocess"`: Flags shell execution with `shell=True`, `os.system(...)`, or `os.popen(...)`.
  - `RULE_PYTHON_PICKLE_DESERIALIZATION: &str = "security/python-pickle-deserialization"`: Flags untrusted deserialization via `pickle.loads(...)` and `_pickle.loads(...)`.
  - `scan_python_unsafe_patterns(relative_path: &str, content: &str) -> Vec<Finding>`: Scans Python source while stripping comments and docstrings.
  - `analyze_python_dependencies(repo_root: &Path, imported_packages: &BTreeSet<String>) -> Result<DependencyReport, ForgeError>`: Detects unpinned versions (`deps/unpinned-version`), duplicate requirements (`deps/duplicate-dependency`), and unused packages.
  - Ecosystem detection: Added native detectors for `FastAPI`, `Flask`, `Django`, and `pytest`.
  - `DEFAULT_IGNORED_DIRS`: Added Python bytecode and virtualenv directories (`__pycache__`, `.pytest_cache`, `.venv`, `venv`).
  - Curated Fixture: `fixtures/fastapi-app/` with FastAPI routing, models, circular import between `app/routers` and `app/services`, unpinned dependency `requests = "*"`, and `pickle.loads` security violation.
  - Golden Test: `crates/forge-cli/tests/fastapi_app_golden_test.rs` validating doctor diagnostics and full scan signal determinism.
- **Invariants:**
  - 100% adherence to zero-panic policy; test assertions return `Result<(), Box<dyn Error>>`.
  - Completes **Epic #149** (Phase 7A: Python Ecosystem Intelligence).


## Issue #154 — go.mod Manifest Parser, Go Import Extractor & Framework Detectors (COMPLETED)
- **Crate:** `crates/forge-analyzer` (`src/go_manifest.rs`, `src/go_imports.rs`, `src/detector.rs`, `src/graph.rs`, `src/lib.rs`, `tests/go_manifest_test.rs`, `tests/go_imports_test.rs`)
- **Public Types & Functions:**
  - `ParsedGoManifest`: struct holding `module_path`, `go_version`, `dependencies: Vec<Dependency>`, and `has_go_sum: bool`.
  - `parse_go_mod_content(content: &str, has_go_sum: bool) -> Result<ParsedGoManifest, ForgeError>`: Parses `module`, `go`, and direct/indirect `require` blocks into normalized `Dependency` structs (`ecosystem: "go"`).
  - `parse_go_manifest(repo_root: &Path) -> Result<ParsedGoManifest, ForgeError>`.
  - `GO_STDLIB_ROOT_PACKAGES`: Immutable catalog of 40+ official Go standard library root packages.
  - `is_go_stdlib(import_path: &str) -> bool`: Filters standard library vs external vs internal Go imports.
  - `strip_go_comments(source: &str) -> String`: Strips `//` line comments and `/* */` block comments while preserving newlines.
  - `extract_go_imports(source: &str) -> Vec<String>`: Extracts single-line and multiline `import (...)` paths.
  - `resolve_go_import(importer_rel_path: &str, raw_import: &str, module_path: Option<&str>, known_files: &BTreeSet<String>) -> Option<String>`: Resolves internal module and relative imports to repository Go files.
  - `extract_go_external_module(raw_import: &str, module_path: Option<&str>) -> Option<String>`: Extracts normalized external module paths (e.g. `github.com/gin-gonic/gin`).
  - `build_go_architecture_graph(repo_root: &Path, files: &[FileNode], module_path: Option<&str>) -> Result<ArchitectureGraph, ForgeError>`: Maps Go packages to logical `Module` boundaries and detects circular dependencies via Tarjan SCC.
  - Ecosystem Detectors: Added Gin (`github.com/gin-gonic/gin`), Echo (`github.com/labstack/echo`), Fiber (`github.com/gofiber/fiber`), Chi (`github.com/go-chi/chi`), and Go package manager `go`.
- **Invariants:**
  - Zero-panic: all parsing operations strictly propagate `ForgeError`.
  - Go standard library imports are filtered out from external dependency findings.

## Issue #155 — Cargo.toml Workspace Dependency Parser & Rust Module Graph Extractor (COMPLETED)
- **Crate:** `crates/forge-analyzer` (`src/rust_manifest.rs`, `src/rust_imports.rs`, `src/graph.rs`, `src/detector.rs`, `src/lib.rs`, `tests/rust_manifest_test.rs`, `tests/rust_imports_test.rs`)
- **Public Types & Functions:**
  - `ParsedRustWorkspace`: Struct holding `is_workspace: bool`, `workspace_members: Vec<String>`, `dependencies: Vec<Dependency>`, `wildcard_dependencies: Vec<String>`, `path_dependencies: Vec<String>`, and `has_cargo_lock: bool`.
  - `ParsedCargoDeps`: Type alias for `(Vec<Dependency>, Vec<String>, Vec<String>)`.
  - `parse_cargo_dependencies(table: &TomlValue, section: &str, is_dev: bool, lock_versions: &BTreeMap<String, String>) -> ParsedCargoDeps`: Parses dependencies under `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, and `[workspace.dependencies]`, flagging wildcards (`*`) and path dependencies.
  - `parse_cargo_toml_content(content: &str, lock_versions: &BTreeMap<String, String>) -> Result<ParsedCargoDeps, ForgeError>`.
  - `parse_rust_workspace(repo_root: &Path) -> Result<ParsedRustWorkspace, ForgeError>`: Deeply parses single-crate projects and multi-member workspaces (`[workspace.members]`), expanding glob patterns (`crates/*`), and merging crate dependencies.
  - `read_cargo_lock_versions(repo_root: &Path) -> BTreeMap<String, String>`: Resolves exact installed versions from root `Cargo.lock`.
  - `RUST_STD_CRATES`: Catalog of Rust standard library crates (`std`, `core`, `alloc`, `proc_macro`, `test`).
  - `is_rust_std(crate_name: &str) -> bool`: Filters standard library crates.
  - `strip_rust_comments(source: &str) -> String`: Strips `//` line comments, doc comments, and nested `/* ... */` block comments while preserving string and raw string literals.
  - `expand_use_tree(prefix: &str, tree: &str) -> Vec<String>`: Recursively expands nested Rust `use` trees, braces, and aliases.
  - `extract_rust_mods_and_uses(source: &str) -> (Vec<String>, Vec<String>)`: Extracts `mod <name>;` declarations and `use` import statements.
  - `resolve_rust_import(importer_rel_path: &str, raw_use: &str, known_files: &BTreeSet<String>) -> Option<String>`: Resolves `crate::...`, `super::...`, `self::...`, declared submodules `mod::<name>`, and workspace crate references.
  - `extract_rust_external_crate(raw_use: &str) -> Option<String>`: Extracts third-party external crates while filtering stdlib and internal modules.
  - `infer_rust_module_boundary(relative_path: &str) -> (String, String)`: Normalizes workspace crate and submodule boundaries.
  - `build_rust_architecture_graph(repo_root: &Path, files: &[FileNode]) -> Result<ArchitectureGraph, ForgeError>`: Builds Rust architecture graph and runs Tarjan SCC cycle detection.
  - Ecosystem detection: Added detection for `axum`, `actix-web`, `tokio`, and `rocket`.
- **Invariants:**
  - 100% adherence to zero-panic policy; test assertions return `Result<(), Box<dyn Error>>`.
  - Self-dogfooding test verified against `elcode-forge` workspace itself.

## Issue #156 — Curated Go (fixtures/go-service) and Rust (fixtures/rust-cli) Fixtures & Golden Integration Tests (COMPLETED)
- **Crates & Fixtures:** `fixtures/go-service/`, `fixtures/rust-cli/`, `crates/forge-cli` (`src/commands/doctor.rs`, `src/commands/scan.rs`, `src/commands/graph.rs`, `tests/go_rust_fixtures_golden_test.rs`), `crates/forge-core` (`src/cache.rs`)
- **Deliverables & Capabilities:**
  - `fixtures/go-service/`: Idiomatic Go microservice with `go.mod` (module `example.com/goservice`), `go.sum`, `cmd/api/main.go`, `internal/handlers/`, `internal/services/`, `internal/storage/`, and `internal/services/user_test.go`. Features an intentional circular package dependency between `internal/handlers` and `internal/services`.
  - `fixtures/rust-cli/`: Modular Rust CLI crate with `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/config.rs`, `src/commands/`, `src/utils/`, and `tests/cli_test.rs`. Features an intentional wildcard dependency (`log = "*"`).
  - Multi-language pipeline wiring in `crates/forge-cli`: `eforge doctor` checks `Go manifest & lockfile hygiene` and `Rust manifest & lockfile hygiene`; `eforge scan` maps Go and Rust architecture graphs and parses dependencies; `eforge graph` supports language-aware Mermaid exports.
  - Race-condition hardening in `crates/forge-core/src/cache.rs`: `save_metadata` ensures parent directories exist prior to atomic writes.
  - Golden integration tests in `crates/forge-cli/tests/go_rust_fixtures_golden_test.rs` validating doctor diagnostics and full scan determinism across Go and Rust fixtures.
- **Invariants:**
  - Strict zero-panic test discipline (`Result<(), Box<dyn Error>>` everywhere).
  - Completes **Epic #153** (Phase 7B: Go & Rust Ecosystems Intelligence).

## Issue #158 — Polyglot Repository Scanner & Unified Architecture Graph Builder (COMPLETED)
- **Crates:** `crates/forge-analyzer` (`src/graph.rs`, `src/dep_health.rs`, `src/lib.rs`, `tests/polyglot_scanner_test.rs`), `crates/forge-cli` (`src/commands/{scan.rs, graph.rs, diff.rs, report.rs, dependencies.rs, doctor.rs}`, `tests/polyglot_golden_test.rs`), `crates/forge-mcp` (`src/tools.rs`)
- **Public Functions Exposed:**
  - `build_polyglot_architecture_graph(repo_root: &Path, files: &[FileNode]) -> Result<ArchitectureGraph, ForgeError>`: Unifies TypeScript, JavaScript, Python, Go, and Rust AST/import graphs into a single namespaced architecture graph, re-evaluating global circular dependencies via Tarjan's SCC and computing high-centrality modules.
  - `analyze_polyglot_dependencies(repo_root: &Path, files: &[FileNode], external_imports: &BTreeSet<String>) -> Result<DependencyReport, ForgeError>`: Recursively discovers and analyzes manifests across npm (`package.json`), PyPI (`pyproject.toml`, `requirements.txt`), Go (`go.mod`), and Cargo (`Cargo.toml`), returning aggregated dependencies and hygiene findings.
- **CLI Commands Parity:**
  - `eforge scan`: Migrated to unified polyglot pipeline, automatically analyzing all detected ecosystems in a single scan pass.
  - `eforge graph`: Generates unified polyglot Mermaid and JSON graphs across all repository languages.
  - `eforge diff` & `eforge report`: Unified graph enables cross-language blast-radius and impact analysis.
  - `eforge dependencies`: Formats ecosystem-labeled dependencies across npm, PyPI, Go, and Cargo.
  - `eforge doctor`: Evaluates hygiene for all present manifests (Node, Python, Rust, Go) in parallel without short-circuiting.
  - `crates/forge-mcp`: `build_analyzed_repo` exposes full polyglot architecture and dependencies to MCP tools.
- **Invariants:**
  - Preserves module namespace isolation across polyglot subprojects (e.g. `frontend/src/components` vs `backend/app/routers`).
  - Zero-panic: all operations propagate `ForgeError`.

## Issue #159 — Web Intelligence Dashboard Multi-Ecosystem Dependency Viewer & Language Badges (COMPLETED)
- **App & Tests:** `apps/dashboard/src/` (`lib/filters.ts`, `routes/dependencies/+page.svelte`, `routes/findings/+page.svelte`, `routes/architecture/+page.svelte`), `apps/dashboard/tests/` (`filters.test.ts`, `dependencies.test.ts`)
- **Capabilities & UI Surfaces:**
  - `apps/dashboard/src/lib/filters.ts`:
    - Added `language?: string` to `FindingFilter` interface and query serialization (`?language=...`).
    - Added `detectFileLanguage(filePath?: string | null): string | null` supporting Python, Go, Rust, TypeScript, JavaScript, Svelte, Vue, and manifests (`Cargo.toml`, `go.mod`, `requirements.txt`, `pyproject.toml`, `package.json`).
  - `apps/dashboard/src/routes/dependencies/+page.svelte`:
    - Multi-ecosystem filter pills (`All`, `npm`, `PyPI`, `Go`, `Cargo`) with package counts and distinct ecosystem badge colors.
    - Dependency type classification badges (`DIRECT`, `DEV`, `INDIRECT`).
    - Search by package name combined with ecosystem and dependency type filtering.
  - `apps/dashboard/src/routes/findings/+page.svelte`:
    - Dynamic language filter pills based on findings present in report.
    - Inline language badges in finding cards next to file locations.
    - Full URL query state synchronization (`?language=...`).
  - `apps/dashboard/src/routes/architecture/+page.svelte`:
    - Primary language inference per architectural module from member files.
    - Language filter toolbar in accessible module table view.
    - Language badges in table rows and module inspector pane.
  - Tests in `apps/dashboard/tests/filters.test.ts` and `apps/dashboard/tests/dependencies.test.ts` validating language detection, query serialization, and multi-ecosystem filtering.
- **Invariants:**
  - Reactive runes adhere to Svelte 5 standards; clean lint and test execution with zero warnings.

## Issue #160 — Multi-Language E2E Integration Suite, Documentation & Workspace v0.7.0 Bump (COMPLETED)
- **Crates & Packages:** `crates/forge-cli/tests/multi_language_e2e.rs`, `README.md`, `docs/MULTI_LANGUAGE.md`, `Cargo.toml`, `package.json`, `apps/dashboard/package.json`, `packages/forge-schema/package.json`, `packages/forge-ui/package.json`, `packages/forge-cli-npm/package.json`
- **Deliverables & Capabilities:**
  - Multi-Language E2E Integration Test Suite (`crates/forge-cli/tests/multi_language_e2e.rs`):
    - Full pipeline execution across all 4 curated fixtures (`fixtures/next-app/`, `fixtures/fastapi-app/`, `fixtures/go-service/`, `fixtures/rust-cli/`).
    - Validates CLI command matrix: `doctor` (manifest hygiene & framework detection), `scan` (polyglot architecture graph, 8 health dimensions, SARIF/terminal/markdown rendering), `review` (blast radius & change risk with synthetic diffs), `explain` (repo & module level structural inspection), and `dependencies` (multi-ecosystem manifest parsing).
    - Cross-language determinism test verifying byte-identical JSON output across successive scans and ensuring dimension score weights sum to 100%.
  - Comprehensive documentation updates:
    - Updated `README.md` with multi-language and ecosystem capability descriptions, version bump to `v0.7.0`, and added the Multi-Language & Ecosystem Support Matrix table.
    - Added [`docs/MULTI_LANGUAGE.md`](./MULTI_LANGUAGE.md) detailing language parsers, import resolution strategies, cycle detection algorithms (Tarjan's SCC), and CLI command parity.
  - Synchronized monorepo workspace version to `v0.7.0` across Rust crates and TypeScript packages.
- **Invariants:**
  - Strict zero-panic policy enforced in all tests (`Result<(), Box<dyn Error>>`).
  - Completes **Epic #157** (Phase 7C) and concludes **Phase 7: Multi-Language Expansion**.

## Issue #171 — Composer Manifest Parser, Lockfile Hygiene & Dependency Health for PHP (COMPLETED)
- **Crates:** `crates/forge-analyzer` (`src/composer_manifest.rs`, `src/detector.rs`, `src/dep_health.rs`, `tests/composer_manifest_test.rs`), `crates/forge-cli` (`src/commands/doctor.rs`)
- **Public Types & Functions (`forge_analyzer::composer_manifest`):**
  - `ParsedComposerManifest`: `name`, `dependencies`, `psr4_mappings`, `unpinned_packages`, `duplicate_packages`, `has_composer_lock`.
  - `parse_composer_manifest(repo_root: &Path) -> Result<ParsedComposerManifest, ForgeError>`.
  - `parse_composer_json_content(content: &str, has_lock: bool, lock_versions: &BTreeMap<String, String>) -> Result<ParsedComposerManifest, ForgeError>`.
  - `read_composer_lock_versions(repo_root: &Path) -> BTreeMap<String, String>`.
  - `is_composer_unpinned(version_req: &str) -> bool`.
  - `analyze_composer_dependencies(repo_root: &Path, external_imports: &BTreeSet<String>) -> Result<DependencyReport, ForgeError>`.
  - `PHP_IMPLICIT_RUNNERS`: runtime catalog for `phpunit`, `pest`, `phpstan`, `psalm`, `rector`, `pint`, `sail`, `faker`, `mockery`, `collision`, `ignition`.
- **Ecosystem & CLI Doctor Integration:**
  - `detect_ecosystem`: Detects package manager `"composer"`, frameworks (`"laravel"`, `"symfony"`, `"wordpress"`), and test runners (`"phpunit"`, `"pest"`).
  - `eforge doctor`: Evaluates `PHP manifest & lockfile hygiene`, flagging missing `composer.lock`, unpinned packages, and duplicate declarations.
  - `analyze_polyglot_dependencies`: Integrates `composer.json` discovery and evaluation across repository subdirectories.
- **Invariants:**
  - Zero panics; all operations propagate `ForgeError`.


## Issue #172 — PHP Namespace & Use Import Graph, Cycle Detection & Security Pattern Scanner (COMPLETED)
- **Crates:** `crates/forge-analyzer` (`src/php_imports.rs`, `src/graph.rs`, `tests/php_imports_test.rs`), `crates/forge-security` (`src/php_patterns.rs`, `src/lib.rs`, `tests/php_security_test.rs`)
- **Public Types & Functions (`forge_analyzer::php_imports`):**
  - `build_php_architecture_graph(repo_root: &Path, files: &[FileNode]) -> Result<ArchitectureGraph, ForgeError>`.
  - `extract_php_namespace_and_uses(source: &str) -> (Option<String>, Vec<String>)`: Extracts namespace declaration and use statements (including aliases and group use statements), filtering out PHP built-in classes (`stdClass`, `DateTime`, `Exception`, `PDO`, etc.).
  - `strip_php_comments(source: &str) -> String`: Strips single-line (`//`, `#`) and multi-line (`/* ... */`) comments while preserving string literals.
  - `infer_php_module_boundary(relative_path: &str) -> String`: Infers architecture module boundaries for Laravel structures (`app/Http/Controllers`, `app/Models`, `app/Services`, `app/Providers`, `tests/Feature`, `tests/Unit`) and PSR-4 directory layouts.
  - `resolve_php_import(target_class: &str, psr4_mappings: &[(String, String)], repo_root: &Path) -> Option<String>`: Resolves PHP class names to file paths using PSR-4 mappings or default `App\` conventions.
  - `extract_composer_package_name(import: &str) -> Option<String>`: Identifies external Composer vendor packages (`Illuminate\` -> `laravel/framework`, `GuzzleHttp\` -> `guzzlehttp/guzzle`, `Symfony\Component\<Name>` -> `symfony/<kebab-name>`).
  - Integrated into `build_polyglot_architecture_graph` in `crates/forge-analyzer/src/graph.rs` with Tarjan's SCC cycle detection.
- **PHP Security Pattern Scanner (`forge_security::php_patterns`):**
  - `scan_php_unsafe_patterns(relative_path: &str, content: &str) -> Vec<Finding>`.
  - Built-in rules:
    - `RULE_PHP_UNSAFE_EVAL` (`security/php-unsafe-eval`, Critical): Flags dynamic code execution via `eval(...)`.
    - `RULE_PHP_INSECURE_UNSERIALIZE` (`security/php-insecure-unserialize`, High): Flags PHP object injection via untrusted `unserialize(...)`.
    - `RULE_PHP_COMMAND_INJECTION` (`security/php-command-injection`, High): Flags arbitrary shell execution via `exec`, `passthru`, `shell_exec`, `system`, `popen`, or `proc_open`.
    - `RULE_PHP_CREATE_FUNCTION` (`security/php-create-function`, High): Flags deprecated runtime code generation via `create_function(...)`.
  - Wired into `SecurityScanner::scan_file_content` for `.php` files with comment filtering.
- **Invariants:**
  - Zero panics across library crates (`unsafe_code = "forbid"`, strict `ForgeError` propagation).


## Issue #173 — Curated Laravel Fixture, Polyglot Integration & Dashboard PHP Badge (COMPLETED)
- **Paths & Crates:** `fixtures/laravel-app/`, `crates/forge-cli/tests/laravel_app_golden_test.rs`, `crates/forge-cli/tests/multi_language_e2e.rs`, `apps/dashboard/src/lib/filters.ts`, `apps/dashboard/src/routes/dependencies/+page.svelte`, `apps/dashboard/src/routes/findings/+page.svelte`, `apps/dashboard/src/routes/architecture/+page.svelte`, `README.md`, `docs/MULTI_LANGUAGE.md`.
- **Deliverables & Capabilities:**
  - Curated Laravel 10 Fixture (`fixtures/laravel-app/`):
    - Idiomatic structure: `composer.json`, `composer.lock`, `artisan`, `phpunit.xml`.
    - Controllers (`app/Http/Controllers/UserController.php`), Services (`app/Services/UserService.php`), Models (`app/Models/User.php`), Feature Tests (`tests/Feature/UserTest.php`).
    - Intentional architectural cycle (`app/Http/Controllers` <-> `app/Services`) and unpinned dependency (`guzzlehttp/guzzle: *`) for deterministic regression testing.
  - Golden E2E Test Suite (`crates/forge-cli/tests/laravel_app_golden_test.rs` & `multi_language_e2e.rs`):
    - Validates full CLI lifecycle: `doctor` (manifest hygiene & framework detection), `scan` (8 health dimensions, circular dependency, unpinned versions, SARIF/MD/terminal output), `review` (synthetic diff impact & risk score), `explain` (module inbound/outbound edges), and `dependencies` (Composer dependencies & lockfile versions).
    - Added `"laravel-app"` to `test_polyglot_cross_language_parity_and_determinism` verifying byte-identical scan output and 100% health score weight invariant across all 5 test repositories.
  - Web Intelligence Dashboard Multi-Ecosystem Experience:
    - Updated `detectFileLanguage` to support `.php`, `composer.json`, and `composer.lock`.
    - Added `Composer` ecosystem filter pill and purple/indigo badges to `/dependencies`.
    - Added `php` badge styling to `/findings` and `/architecture`.
    - Unit tests in `apps/dashboard/tests/filters.test.ts` and `apps/dashboard/tests/dependencies.test.ts`.
  - Comprehensive documentation updates in `README.md` and `docs/MULTI_LANGUAGE.md` reflecting PHP, Composer, and Laravel ecosystem support.
- **Invariants:**
  - Zero panics; strict error propagation (`Result<(), Box<dyn Error>>` in tests).
  - Completes **Epic #170** (Phase 7D: PHP & Laravel Ecosystem Intelligence).


## Issue #183 — `forge-security` OSV Advisory Client, Offline Cache & Polyglot Vulnerability Matcher (COMPLETED)
- **Crate:** `crates/forge-security` (`src/osv/{mod.rs, version.rs, cache.rs, matcher.rs}`, `src/lib.rs`, `tests/osv_test.rs`)
- **Public Types & Functions (`forge_security::osv`):**
  - `OsvAdvisory`, `OsvPackage`, `OsvRange`, `OsvEvent`, `OsvSeverity`, `LocalAdvisoryCache`, `Version`, `VulnerabilityMatcher`, `RULE_VULNERABLE_DEPENDENCY` (`"security/vulnerable-dependency"`).
  - `SecurityScanner::scan_dependencies(repo_root: &Path, dependencies: &[Dependency]) -> Vec<Finding>`.
  - `VulnerabilityMatcher::load(repo_root: &Path) -> Self`.
  - `VulnerabilityMatcher::match_dependency(&self, dep: &Dependency) -> Vec<Finding>`.
  - `VulnerabilityMatcher::match_dependencies(&self, deps: &[Dependency]) -> Vec<Finding>`.
  - `normalize_ecosystem(ecosystem: &str) -> &'static str`, `normalize_package_name(ecosystem: &str, name: &str) -> String`.
- **Capabilities & Deliverables:**
  - Robust zero-panic semver & ecosystem version parser (`Version`) supporting standard SemVer 2.0.0, CalVer, and non-strict release strings without external crate dependencies.
  - OSV v1 schema deserializer and CVSS v3 score / label mapping to normalized `Severity` (`Critical`, `High`, `Medium`, `Low`).
  - Offline local cache in `.eforge/cache/osv/<ecosystem>.json` combined with an embedded offline catalog of vetted CVEs covering npm (`lodash`, `minimist`), PyPI (`requests`, `urllib3`), Go (`gin`), Cargo (`log`), and Composer (`guzzlehttp/guzzle`).
  - In-memory indexed vulnerability matcher evaluating package version intervals (`introduced`..`fixed`/`last_affected`), generating deterministic `Finding` records with actionable upgrade recommendations.
- **Invariants:**
  - Zero-panic: `unsafe_code = "forbid"`, all I/O and serialization errors map to `ForgeError::Io` and `ForgeError::Serialization`.
  - Deterministic finding sorting by `(Severity::desc, id::asc)`.
  - Contributes to **Epic #182** (Phase 8A: Vulnerability Intelligence & OSV/Advisory Engine).


## Issue #184 — `forge-cli` `eforge security --audit`, SARIF Vulnerability Rules & CI Gate (COMPLETED)
- **Crates:** `crates/forge-cli` (`src/cli.rs`, `src/commands/security.rs`, `src/commands/scan.rs`, `src/formatters/sarif.rs`, `src/formatters/mod.rs`, `src/lib.rs`, `tests/security_command_test.rs`)
- **Public CLI & Functions Exposed:**
  - `eforge security [--audit] [--offline] [--fail-on <severity>]`: Enhanced security subcommand auditing polyglot package dependencies against OSV advisories.
  - `commands::security::run_security(repo_root: &Path, format: OutputFormat, audit: bool, offline: bool, fail_on: Option<&str>) -> Result<(String, ExitCode), ForgeError>`.
  - `render_sarif`: Re-exported in `forge_cli::formatters::render_sarif` and updated with CVSS v3 `security-severity` scores (9.0/7.0/4.0/2.0), vulnerability tags (`["security", "vulnerability", "supply-chain"]`), and CVE/GHSA reference `helpUri` links.
  - `execute_scan`: Automatically includes `SecurityScanner::scan_dependencies` findings in repository findings and 8-dimension health score calculations.
- **Invariants:**
  - Zero-panic: all arguments and thresholds propagate structured errors and deterministic `ExitCode` values (`ExitCode::SecurityPolicyFailure` = 4 on security threshold breach).
  - Terminal format for `eforge security --audit` renders structured severity breakdown (`Critical`, `High`, `Medium`, `Low`) and actionable remediation guidance.
  - Contributes to **Epic #182** (Phase 8A: Vulnerability Intelligence & OSV/Advisory Engine).


## Issue #185 — Curated Vulnerable Dependencies Fixture & Golden Test Suite (COMPLETED)
- **Paths & Crates:** `fixtures/vulnerable-dependencies/`, `crates/forge-security/src/osv/matcher.rs`, `crates/forge-cli/tests/security_vulnerability_golden_test.rs`
- **Deliverables & Capabilities:**
  - `fixtures/vulnerable-dependencies/`: Multi-ecosystem test fixture containing pinned vulnerable dependencies with public CVEs:
    - npm: `package.json` & `package-lock.json` with `lodash@4.17.15` (CVE-2020-8203, High) and `minimist@1.2.5` (CVE-2021-44906, Critical).
    - PyPI: `requirements.txt` with `requests==2.19.1` (CVE-2018-18074, High).
    - Go: `go.mod` & `go.sum` with `github.com/gin-gonic/gin@v1.6.0` (CVE-2020-28483, High).
    - Composer: `composer.json` & `composer.lock` with `guzzlehttp/guzzle@7.4.0` (CVE-2022-29248, High).
    - Polyglot source files: `src/index.js`, `app.py`, `cmd/main.go`, `index.php`, and `README.md`.
  - Prefix-stripping resilience in `crates/forge-security/src/osv/matcher.rs`: Enhanced `clean_version_requirement` to strip multiple operator characters (e.g. `==2.19.1` -> `2.19.1`).
  - Golden integration test suite (`crates/forge-cli/tests/security_vulnerability_golden_test.rs`):
    - Validates full vulnerability audit lifecycle across all 5 pinned CVEs.
    - Asserts deterministic byte-for-byte JSON output across successive scans.
    - Validates policy gate exit codes (`ExitCode::SecurityPolicyFailure` = 4 on `--fail-on high` and `--fail-on critical`).
    - Validates OASIS SARIF v2.1.0 output formatting with CVSS `security-severity` scores.
    - Confirms `execute_scan` automatically integrates vulnerability findings and depresses the `Security` dimension score (< 50).
- **Invariants:**
  - Zero-panic test discipline (`Result<(), Box<dyn Error>>` with `?` operator).
  - Completes **Epic #182** (Phase 8A: Vulnerability Intelligence & OSV/Advisory Engine).


## Issue #190 — CycloneDX v1.5 & SPDX v2.3 JSON SBOM Generators (`eforge sbom`) (COMPLETED)
- **Crates:** `crates/forge-security` (`src/sbom/mod.rs`, `src/sbom/purl.rs`, `src/sbom/cyclonedx.rs`, `src/sbom/spdx.rs`, `src/lib.rs`), `crates/forge-cli` (`src/cli.rs`, `src/commands/sbom.rs`, `src/commands/mod.rs`, `src/lib.rs`, `tests/sbom_command_test.rs`).
- **Public Types & Functions Exposed:**
  - `forge_security::sbom::{generate_sbom, generate_cyclonedx_sbom, generate_spdx_sbom, generate_purl, SbomFormat}`: Multi-ecosystem SBOM generators exporting CycloneDX v1.5 JSON and SPDX v2.3 JSON.
  - Canonical Package URL (PURL) generation for npm, PyPI, Go, Cargo, and Composer (`pkg:<eco>/...`).
  - Deterministic BLAKE3 serial numbers for CycloneDX documents (`urn:uuid:...`) and document namespaces for SPDX.
  - `eforge sbom [--format cyclonedx|spdx] [-o <path>]`: CLI subcommand generating standard SBOMs for polyglot repositories with automatic project identity and version discovery from `package.json`, `Cargo.toml`, and `composer.json`.
- **Invariants:**
  - Zero-panic: `unsafe_code = "forbid"`, denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`.
  - Deterministic sorting by ecosystem, package name, and version requirement across all components.
  - Contributes to **Epic #189** (Phase 8B: SBOM & License Compliance Engine).


## Issue #191 — Open-Source License Detector & License Compatibility Policy Checker (COMPLETED)
- **Crates:** `crates/forge-core` (`src/config.rs`, `src/lib.rs`), `crates/forge-security` (`src/license/category.rs`, `src/license/detector.rs`, `src/license/checker.rs`, `src/license/mod.rs`, `src/lib.rs`, `tests/license_test.rs`), `crates/forge-cli` (`src/commands/dependencies.rs`, `src/commands/scan.rs`, `tests/license_cli_test.rs`).
- **Public Types & Functions Exposed:**
  - `forge_core::config::LicenseConfig`: Declares `disallow_copyleft`, `allowlist`, and `denylist` in `eforge.config.yaml`.
  - `forge_security::license::{LicenseCategory, categorize_license, detect_project_license, detect_dependency_license, evaluate_license_compliance}`: Polyglot license classifier (`Permissive`, `WeakCopyleft`, `StrongCopyleft`, `Proprietary`, `Unknown`), manifest and license text detector, and policy evaluator.
  - License Rules: `license/disallowed-copyleft` (High), `license/denied-license` (High), `license/unapproved-license` (Medium), `license/missing-project-license` (Low).
  - CLI Integration: `eforge dependencies` displays project and package license information across terminal, markdown, and JSON outputs; `eforge scan` includes license policy violations in repository health calculation and gate enforcement.
- **Invariants:**
  - Zero-panic: `unsafe_code = "forbid"`, denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`.
  - Weak copyleft licenses (LGPL, MPL, etc.) evaluated before strong copyleft to prevent false positive copyleft contamination.
  - Contributes to **Epic #189** (Phase 8B: SBOM & License Compliance Engine).


## Issue #192 — Web Dashboard Vulnerability Inspector, SBOM Visualizer & License Compliance (COMPLETED)
- **Apps:** `apps/dashboard` (`src/routes/security/+page.svelte`, `src/routes/dependencies/+page.svelte`, `src/lib/data/license.ts`, `src/lib/data/sbom.ts`, `tests/license_sbom.test.ts`, `tests/security.test.ts`).
- **Features & Capabilities:**
  - Security & Vulnerability Screen: KPI cards, CVE/GHSA advisory inspector with external NVD/GHSA links, actionable upgrade command generator with 1-click copy, and multi-tier filters (severity, category, ecosystem, search).
  - Open-Source License Distribution Bar & Summary: Visual ratio of Permissive, Weak Copyleft, Strong Copyleft, Proprietary, and Unknown licenses.
  - License-Aware Dependencies Table: Inline category badges and filter by license tier.
  - One-Click SBOM Exporter: Direct browser download for CycloneDX v1.5 JSON and SPDX v2.3 JSON generated from loaded `ReportBundle`.
- **Invariants:**
  - Pure client-side generation without external runtime network calls.
  - Completes **Epic #189** (Phase 8B: SBOM & License Compliance Engine).

## Issue #197 — Shannon Entropy Scanner, Verified Credential Patterns & Path Allowlist (COMPLETED)
- **Crates:** `crates/forge-core` (`src/config.rs`, `src/lib.rs`), `crates/forge-security` (`src/entropy.rs`, `src/secrets.rs`, `src/lib.rs`, `tests/entropy_test.rs`), `crates/forge-cli` (`src/commands/security.rs`, `src/commands/scan.rs`, `src/commands/doctor.rs`), `crates/forge-mcp` (`src/tools.rs`).
- **Public Types & Functions Exposed:**
  - `forge_core::config::SecurityConfig`: Declares `allowlist`, `entropy_threshold`, and `entropy_min_len` under `security:` in `eforge.config.yaml`.
  - `forge_security::entropy::{calculate_shannon_entropy, is_high_entropy, is_hex, tokenize_candidate_secrets, find_high_entropy_tokens}`: Shannon entropy calculation engine ($H(X) = -\sum P(x)\log_2 P(x)$) with proportional hex threshold scaling.
  - `forge_security::secrets::{scan_secrets_with_config, is_lockfile, is_path_allowlisted, RULE_HIGH_ENTROPY_SECRET}`: Verified credential patterns (AWS, GitHub classic and fine-grained, Slack bot and user tokens, OpenAI, Anthropic) and generic high-entropy secret detection (`security/high-entropy-secret`).
  - Mandatory `****REDACTED****` masking in finding evidence and prompt preparation (`redact_secrets`).
  - Security path allowlist & automatic lockfile filtering to eliminate false positives in package manifests and test fixtures.
- **Invariants:**
  - Zero-panic: `unsafe_code = "forbid"`, denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`.
  - Redaction is strictly enforced before findings are persisted or logged.
  - Contributes to **Epic #196** (Phase 8C: Advanced Entropy Secret Detection & v0.8.0 Release Prep).

## Issue #198 — Multi-Ecosystem Security E2E Test Suite, Documentation & Workspace v0.8.0 Bump (COMPLETED)
- **Files & Crates:** `crates/forge-cli` (`tests/security_e2e_test.rs`), `docs/SECURITY_GUIDE.md`, `README.md`, `Cargo.toml`, `package.json`, `apps/dashboard/package.json`, `packages/*/package.json`.
- **E2E Test Suite:**
  - `test_security_e2e_vulnerability_and_sarif_pipeline`: Full end-to-end OSV vulnerability auditing across multi-ecosystem manifests (npm, PyPI) with OASIS SARIF v2.1.0 JSON exports and policy gate enforcement.
  - `test_security_e2e_sbom_cyclonedx_and_spdx`: Software Bill of Materials generation validating CycloneDX v1.5 JSON and SPDX v2.3 JSON structure, canonical PURLs (`pkg:<eco>/...`), and deterministic output across runs.
  - `test_security_e2e_entropy_secret_and_allowlist`: Shannon entropy candidate token analysis, verified cloud credential patterns (AWS, GitHub, Slack, OpenAI, Anthropic), path allowlist bypass, and mandatory `****REDACTED****` evidence masking.
  - `test_security_e2e_license_policy_compliance`: Evaluates copyleft and license allowlist policies during end-to-end scan pipeline.
- **Documentation & Version Bump:**
  - Authored authoritative `docs/SECURITY_GUIDE.md` covering local-first vulnerability audits, SBOM exports, license compliance, Shannon entropy tuning, path allowlists, and GitHub Actions SARIF integration.
  - Updated `README.md` with deep security and supply-chain durability capabilities.
  - Synchronized monorepo workspace version to `v0.8.0` across Rust workspace crates and TypeScript packages.
- **Invariants:**
  - Zero-panic test discipline (`Result<(), Box<dyn Error>>`).
  - Completes **Epic #196** (Phase 8C: Advanced Entropy Secret Detection & v0.8.0 Release Prep) and marks **Phase 8 (Deep Security & Supply-Chain Durability)** 100% completed.



## Issue #202 — Remote Sync Schema, Credential Manager & Content-Addressed Sync Protocol (COMPLETED)
- **Crate:** `crates/forge-core` (`src/error.rs`, `src/sync/mod.rs`, `src/sync/protocol.rs`, `src/sync/credentials.rs`, `src/lib.rs`, `tests/sync_test.rs`).
- **Public Types & Functions Exposed:**
  - `forge_core::sync::protocol::{SyncStatus, SyncManifest, SyncPushPayload, SyncPullResponse, DeltaSummary}`: Serializable, schema-derived synchronization protocol structures.
  - `forge_core::sync::credentials::{ServerCredential, CredentialsConfig, load_credentials, load_credentials_from_path, save_credentials, save_credentials_to_path, resolve_token, home_dir, default_credentials_path}`: Secure credential manager supporting `~/.eforge/credentials.yaml`, `EFORGE_CREDENTIALS_PATH`, and precedence-driven token resolution (`EFORGE_API_TOKEN` > specific server > default server > single configured server).
  - `forge_core::sync::compute_sync_content_hash(files, findings) -> String`: Content-addressed BLAKE3 hashing engine ensuring byte-for-byte reproducibility regardless of collection traversal order.
  - `forge_core::sync::DeltaSummary::compute`: Differential summary engine calculating added, resolved, and unchanged findings, health delta, and change risk shift between local state and remote baseline.
  - `forge_core::error::ForgeError::Sync`: Semantic error variant for remote sync operations.
- **Invariants:**
  - `forge-core` remains a leaf crate with zero internal `forge-*` dependencies.
  - Zero-panic: all file I/O and parsing operations propagate `ForgeError::Io`, `ForgeError::Config`, or `ForgeError::Serialization`.
  - Credentials file permissions on Unix set to `0o600`.
  - Contributes to **Epic #201** (Phase 9A: Remote Sync Protocol & CLI Client).


## Issue #203 — CLI Remote Commands (login, logout, auth, push, pull, sync) (COMPLETED)
- **Crate:** `crates/forge-cli` (`Cargo.toml`, `src/cli.rs`, `src/commands/auth.rs`, `src/commands/sync.rs`, `src/commands/mod.rs`, `src/lib.rs`, `tests/sync_cli_test.rs`).
- **Commands & Capabilities:**
  - `eforge login [--server <url>] [--token <tok>] [--username <user>] [--no-validate]`: Authenticates with remote server, validates against `/api/v1/health`, and stores credentials in `~/.eforge/credentials.yaml`.
  - `eforge logout [--server <url>]`: Clears stored credentials for specified or default remote server.
  - `eforge auth status [--server <url>] [--format <fmt>]`: Displays active server, masked token, credential store source, and all configured servers in Terminal or JSON.
  - `eforge push [--server <url>] [--token <tok>] [--branch <branch>]`: Uploads complete repository snapshot (`SyncPushPayload`) to remote server (`POST /api/v1/sync/push`) and updates local cached baseline.
  - `eforge pull [--server <url>] [--token <tok>] [--branch <branch>]`: Fetches remote baseline snapshot (`GET /api/v1/sync/pull`) and caches it in `<repo_root>/.eforge/baseline.json`.
  - `eforge sync [--server <url>] [--token <tok>] [--branch <branch>] [--fail-on <sev>]`: CI/CD automation pipeline comparing local findings against remote baseline via `DeltaSummary`, pushing updated snapshot, and evaluating policy thresholds.
- **Invariants:**
  - Zero-panic: non-test code propagates structured errors without unwraps.
  - Local cached baseline always written to `<repo_root>/.eforge/baseline.json`.
  - Contributes to **Epic #201** (Phase 9A: Remote Sync Protocol & CLI Client).


## Issue #204 — Remote AST & Dependency Cache Sharing & eforge diff --remote (COMPLETED)
- **Crates:** `crates/forge-core` (`src/cache.rs`, `src/lib.rs`, `tests/remote_cache_test.rs`), `crates/forge-analyzer` (`src/graph.rs`, `src/dep_health.rs`, `src/lib.rs`, `tests/cached_analysis_test.rs`), `crates/forge-cli` (`src/cli.rs`, `src/commands/diff.rs`, `src/commands/scan.rs`, `src/lib.rs`, `src/main.rs`, `src/bin/elcode-forge.rs`, `tests/diff_remote_test.rs`).
- **Public Types & Functions Exposed:**
  - `forge_core::cache::RemoteCacheAdapter`: Trait for remote byte caching (`get_bytes`, `put_bytes`).
  - `forge_core::cache::{InMemoryRemoteCacheAdapter, FileRemoteCacheAdapter, HttpRemoteCacheAdapter}`: Built-in local and remote cache storage adapters.
  - `forge_core::cache::compute_analysis_cache_key(prefix, config_hash, files) -> String`: Content-addressed BLAKE3 hashing engine based on sorted file relative paths, content digests, configuration hash, and engine version.
  - `forge_core::cache::ForgeCache::{get_cached, put_cached, get_with_remote, put_with_remote}`: Object caching methods with transparent fallback and local population.
  - `forge_analyzer::{build_polyglot_architecture_graph_cached, analyze_polyglot_dependencies_cached}`: Cached polyglot graph construction and dependency health analysis with warm cache retrieval (< 500ms).
  - `eforge diff --remote [--format <fmt>]`: Baseline differential comparison between current repository state and cached/pulled remote snapshot from `.eforge/baseline.json`.
  - CLI binary main thread stack expansion (8 MiB) preventing Windows debug thread stack overflow.
- **Invariants:**
  - Zero-panic: non-test code propagates structured errors without unwraps.
  - Cache invalidation is deterministic: changes to source file digests, config hash, or engine version produce unique cache keys.
  - Completes **Epic #201** (Phase 9A: Remote Sync Protocol & CLI Client).


## Issue #206 — Bootstrap crates/forge-server with Axum, SQLite/PostgreSQL & JWT Auth (COMPLETED)
- **Crate:** `crates/forge-server` (`Cargo.toml`, `src/lib.rs`, `src/main.rs`, `src/error.rs`, `src/config.rs`, `src/db/mod.rs`, `src/db/models.rs`, `src/auth/mod.rs`, `src/auth/jwt.rs`, `src/routes/mod.rs`, `src/routes/health.rs`, `src/routes/auth.rs`, `src/routes/sync.rs`, `src/state.rs`, `src/app.rs`, `tests/server_api_test.rs`).
- **Public Types & Functions Exposed:**
  - `forge_server::ServerConfig`: Runtime configuration loader with environment variable fallbacks (`EFORGE_SERVER_HOST`, `EFORGE_SERVER_PORT`, `EFORGE_DATABASE_URL`, `EFORGE_JWT_SECRET`, `EFORGE_ADMIN_API_KEY`).
  - `forge_server::Db`: Dual database connection pool supporting embedded SQLite (with in-memory `sqlite::memory:` mode) and external PostgreSQL with automatic migrations (`projects`, `snapshots`, `findings`, `api_keys`).
  - `forge_server::auth`: Token management utilities (`issue_jwt`, `verify_jwt`, `hash_api_key`) and `AuthenticatedUser` Axum request extractor.
  - `forge_server::create_app`: Configures Axum router with `/api/v1/health`, `/api/v1/auth/token`, `/api/v1/sync/push`, `/api/v1/sync/pull`, permissive CORS, and structured tracing.
  - `forge_server::run_server`: Binds TCP listener and runs Axum server with graceful shutdown handling.
- **Invariants:**
  - Zero-panic: non-test code strictly avoids `unwrap()`, `expect()`, or `panic!()`, returning `ServerError` mapped to proper HTTP status codes (400, 401, 404, 500).
  - In-memory SQLite execution enables zero-dependency integration tests without external database setup.
  - Contributes to **Epic #205** (Phase 9B: Lightweight Self-Hostable Forge Server).


## Issue #207 — Project Workspace Registry, Multi-Branch History Retention & Webhook Dispatcher (COMPLETED)
- **Crate:** `crates/forge-server` (`src/db/mod.rs`, `src/db/models.rs`, `src/projects/mod.rs`, `src/history/mod.rs`, `src/webhooks/mod.rs`, `src/routes/projects.rs`, `src/routes/history.rs`, `src/routes/webhooks.rs`, `src/routes/sync.rs`, `src/app.rs`, `src/lib.rs`, `tests/webhook_and_projects_test.rs`).
- **Public Types & Functions Exposed:**
  - `forge_server::projects::{get_project_overview, ProjectOverview}`: Project workspace overview engine aggregating branch counts, latest health score, open/critical vulnerability counts, and chronological history.
  - `forge_server::history::{calculate_history_trend, HistoryTrend, TrendDirection}`: Historical score retention and statistical trend calculator computing overall health deltas, directional trajectory (`Improving`, `Degrading`, `Stable`), and dimension averages (`architecture_avg`, `security_avg`, `maintainability_avg`).
  - `forge_server::webhooks::{WebhookEvent, WebhookPayload, EventEvaluationContext, generate_hmac_signature, verify_hmac_signature, format_slack_payload, format_discord_payload, dispatch_webhooks, evaluate_and_dispatch_events}`: Outbound webhook notification dispatcher supporting Slack, Discord, and generic HTTP endpoints with HMAC-SHA256 signatures (`X-Forge-Signature: sha256=<hex>`), event triggers (`health_score_drop`, `critical_vulnerability`, `license_violation`, `snapshot_pushed`).
  - API Endpoints: `GET /api/v1/projects`, `GET /api/v1/projects/:repo_id/overview`, `GET /api/v1/projects/:repo_id/branches`, `GET /api/v1/projects/:repo_id/history`, `POST /api/v1/webhooks`, `GET /api/v1/webhooks`, `DELETE /api/v1/webhooks/:id`.
  - Automatic evaluation and asynchronous webhook dispatch wired directly into `POST /api/v1/sync/push`.
- **Invariants:**
  - Zero-panic: all database row mappings and endpoint handlers propagate structured `ServerError` without unwraps.
  - HMAC-SHA256 signatures computed with standard hexadecimal encoding and timing-safe comparison.
  - Contributes to **Epic #205** (Phase 9B: Lightweight Self-Hostable Forge Server).


## Issue #208 — Dockerfile, Docker Compose, Kubernetes Manifests & Self-Hosting Guide (COMPLETED)
- **Files:** `docker/Dockerfile.server`, `docker/Dockerfile.dashboard`, `docker/nginx-dashboard.conf`, `.dockerignore`, `docker-compose.yml`, `.env.example`, `deploy/k8s/{namespace,configmap,secret,deployment,service,ingress,kustomization}.yaml`, `docs/SELF_HOSTING.md`.
- **Capabilities Delivered:**
  - Production multi-stage `Dockerfile.server` producing static, unprivileged Alpine container (< 30 MB) running as `UID 10001` (`forge:forge`) with `/api/v1/health` healthcheck probe.
  - Production `docker-compose.yml` multi-service orchestration (`forge-server`, `postgres`, `minio`, `dashboard`) with persistent volumes and healthcheck dependencies.
  - Cloud-native Kubernetes manifests (`deploy/k8s/`) with HA rolling update deployment, non-root security context, read-only root filesystem, liveness/readiness probes, and ingress routing.
  - Comprehensive operational runbook (`docs/SELF_HOSTING.md`): sizing guidelines, 1-command startup, PostgreSQL/SQLite backups, Nginx/Caddy SSL reverse proxy, and air-gapped installation.
- **Invariants:**
  - Container execution strictly enforces non-root unprivileged user (`UID 10001`).
  - All healthchecks point to standard `/api/v1/health` endpoint.
  - Completes **Epic #205** (Phase 9B: Lightweight Self-Hostable Forge Server).


## Issue #210 — Team Cloud Mode, Project Switcher, Remote Trends & PR Baseline Diff Inspector (COMPLETED)
- **App:** `apps/dashboard` (`src/lib/data/server.ts`, `src/lib/data/source.ts`, `src/lib/data/store.svelte.ts`, `src/routes/+layout.svelte`, `src/routes/projects/+page.svelte`, `src/routes/compare/+page.svelte`, `tests/server_sync.test.ts`).
- **Capabilities Delivered:**
  - `apps/dashboard/src/lib/data/server.ts`: Forge Server client library providing connection testing, project querying, project overview inspection, branch listing, historical score fetching, remote baseline snapshot pulling with transformation to `ReportBundle`, and `computeBaselineDiff` calculation.
  - Mode Switcher & Navigation: Integrated toggle between `Local File` mode and `Team Cloud` mode in header with server status indicator, connection modal, and navigation links for `Projects` and `Compare`.
  - Project Workspace Studio (`/projects`): Multi-repository grid displaying connected projects with real-time health score rings, open findings counters, branch/commit tracking, search filtering, and one-click "Open Dashboard" loading.
  - PR Baseline Diff Inspector (`/compare`): Visual delta inspector calculating health score shifts, added findings, resolved findings, persistent baseline findings, and per-dimension delta bars with severity filtering.
- **Invariants:**
  - Sensitive server tokens are persisted only in local browser `localStorage` under `eforge_server_config`.
  - Contributes to **Epic #209** (Phase 9C: Collaborative Team Intelligence, Dashboard Cloud Sync & v0.9.0 Release Prep).


## Issue #211 — Forge Server CI Integration Test, docs/SELF_HOSTING.md & v0.9.0 Release Prep (COMPLETED)
- **Files:** `.github/workflows/ci-server.yml`, `crates/forge-cli/tests/sync_e2e_test.rs`, `crates/forge-server/tests/postgres_integration_test.rs`, `Cargo.toml`, `package.json`, `packages/forge-schema/package.json`, `packages/forge-ui/package.json`, `packages/forge-cli-npm/package.json`, `apps/dashboard/package.json`, `README.md`, `docs/ARCHITECTURE_STATE.md`.
- **Capabilities Delivered:**
  - Automated CI Workflow (`.github/workflows/ci-server.yml`): Path-aware execution testing both SQLite embedded backend and live PostgreSQL 16 container service with container healthcheck probing.
  - Sync E2E Test Suite (`crates/forge-cli/tests/sync_e2e_test.rs`): Full lifecycle integration test spawning `forge-server` in background on ephemeral port, executing `run_auth_login`, `run_auth_status`, `run_sync_push`, caching baseline, `run_sync_pull`, delta sync pipeline execution, and `run_auth_logout`.
  - PostgreSQL Integration Test Suite (`crates/forge-server/tests/postgres_integration_test.rs`): Dedicated database migration and CRUD lifecycle tests connecting directly to PostgreSQL instances when `FORGE_DATABASE_URL` is set in CI.
  - Workspace Monorepo Version Bump to `0.9.0`: Synchronized workspace version across Rust workspace crates (`Cargo.toml`, `Cargo.lock`) and TypeScript packages (`package.json`, `packages/forge-schema`, `packages/forge-ui`, `packages/forge-cli-npm`, `apps/dashboard`).
  - Documentation Finalization: Updated `README.md` to `v0.9.0` with Team Cloud Sync (`eforge sync`, `eforge push`, `eforge pull`), Self-Hosting architecture links, 23 subcommands CLI table, and CI workflow badges.
- **Invariants:**
  - Synchronous `ureq` CLI invocations in Tokio runtime must execute inside `tokio::task::spawn_blocking` with multi-thread worker pools to prevent loopback socket deadlocks.
  - Completes **Epic #209** (Phase 9C: Collaborative Team Intelligence, Dashboard Cloud Sync & v0.9.0 Release Prep) and marks **Phase 9 (Collaborative Team Intelligence, Remote Sync & Forge Server)** 100% completed.


## Issue #222 — Policy-as-Code Gate Evaluation Engine & Policy Schema (COMPLETED)
- **Crates:** `crates/forge-core` (`src/policy.rs`, `src/config.rs`, `src/lib.rs`, `tests/policy_test.rs`), `crates/forge-git` (`src/impact.rs`), `crates/forge-rules` (`src/policy_engine.rs`, `src/lib.rs`, `tests/policy_test.rs`).
- **Public Types & Functions Exposed:**
  - `forge_core::policy`: `PolicyDocument`, `PolicyConstraints`, `PolicyWaiver` (`is_expired`, `is_expired_at`, `matches`), `PolicyRuleOverride`, `PolicyVerdict`, `PolicyReport` (`is_passed`, `is_violation`, `is_warning`), `PolicyImpactSummary`, `load_policy(path: &Path) -> Result<PolicyDocument, ForgeError>`, `parse_policy_yaml(content: &str) -> Result<PolicyDocument, ForgeError>`, `glob_path_match(pattern: &str, path: &str) -> bool`.
  - `forge_core::config`: Added `pub policy: Option<PolicyDocument>` field to `ForgeConfig`, with automatic YAML block parsing under `policy:`.
  - `forge_git::impact`: `impl From<&ChangeImpactReport> for PolicyImpactSummary` bridging Git blast radius without crate DAG circularity.
  - `forge_rules::policy_engine`: `evaluate_policy(policy: &PolicyDocument, findings: &[Finding], health: &HealthScore, impact: Option<&PolicyImpactSummary>, dependencies: &[Dependency], configured_rule_packs: &[String]) -> PolicyReport`.
- **Invariants:**
  - Crate DAG strictness: `forge-rules` never depends on `forge-git`; `PolicyImpactSummary` in `forge-core` acts as normalized metric carrier.
  - Waivers: time-bounded with RFC 3339 / ISO date checking against UTC, path-scoped via glob matching.
  - Contributes to **Epic #221** (Phase 10A: Organization Policy-as-Code Engine & Compliance Audits).


## Issue #223 — `eforge policy check` CLI Command & Enterprise Gate Exit Codes (COMPLETED)
- **Crate:** `crates/forge-cli` (`src/commands/policy.rs`, `src/commands/mod.rs`, `src/cli.rs`, `src/lib.rs`, `tests/policy_cli_test.rs`).
- **Public Types & Functions Exposed:**
  - `forge_cli::commands::policy`: `execute_policy_check(repo_root: &Path, policy_path: Option<&Path>) -> Result<PolicyReport, ForgeError>`, `compute_policy_exit_code(report: &PolicyReport, fail_on: Option<Severity>) -> ExitCode`, `render_policy_report(report: &PolicyReport, format: OutputFormat) -> Result<String, ForgeError>`, `run_policy_check(repo_root: &Path, policy_path: Option<&Path>, format: OutputFormat, fail_on: Option<Severity>) -> ExitCode`.
  - `forge_cli::cli`: `Commands::Policy`, `PolicyAction::Check` with `--policy <path>`, `--path <dir>`, `--fail-on <severity>`, and `--format <format>`.
- **Invariants:**
  - Policy discovery hierarchy: CLI `--policy <path>` takes precedence over inline `policy:` block in `eforge.config.yaml`. Absence of both yields `ExitCode::ConfigurationError` (2).
  - Deterministic exit codes: `0` (Success/Passed), `4` (SecurityPolicyFailure / Violation), `1` (FindingsAboveThreshold / Warning when threshold met).
  - Contributes to **Epic #221** (Phase 10A: Organization Policy-as-Code Engine & Compliance Audits).


