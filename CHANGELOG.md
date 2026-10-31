# Changelog

All notable changes to **ELCODE Forge** (`eforge`) are documented in this file.
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) and [Conventional Commits](https://www.conventionalcommits.org/).

---

## [1.1.0] — 2026-10-31 (Tactile Instrument, Active Remediation & Graph Heuristics)

### Summary of v1.1.0 Release
ELCODE Forge `v1.1.0` elevates `eforge` from a passive diagnostic scanner into an active, tactile engineering instrument across 3 pillars: **High-Craft Terminal DX**, **Active Remediation & Temporal Graph**, and **Graph-Theoretic Heuristics & Closed-Loop Agentic MCP**.

---

### Pillar 1: High-Craft DX ("Tactile Instrument" in the Terminal)
- **Interactive Terminal UI (`eforge ui` & `--interactive` / `-i` on `doctor`, `scan`, `review`)**:
  - Real-time `ratatui` + `crossterm` terminal dashboard with 4 interactive tabs (`[1] DAG Explorer`, `[2] Tarjan SCC Cycles`, `[3] Blast Radius`, `[4] Findings & Code-Frame`), search filtering (`/`), and full snapshot support (`render_ui_snapshot`) for non-TTY CI environments.
- **Rich Code-Frame Diagnostic Formatter (`--codeframe` on `doctor` and `scan`)**:
  - Box-drawn multi-line source spans (`╭─[...]`, `│`, `╰──▶`), inline carets (`^`), visual call-chain arrows for Tarjan SCC circular dependencies, and unified diff (`-` / `+`) autofix previews.
- **Live Incremental Architectural Radar (`eforge watch`)**:
  - Sub-50ms incremental `blake3` file-level AST caching (`.eforge/cache/file-ast-v1.json`) that re-parses only modified files on save and computes real-time 8-Dimension Health Score deltas (`--once` supported for deterministic single-cycle verification).

### Pillar 2: Deep Engineering (Active Remediation & Temporal Graph)
- **Surgical Auto-Fix & Transactional Rollback (`eforge fix`)**:
  - Deterministic codemod planner and transactional applier (`--dry-run`, `--apply`, `--rule <id>`, `--rollback [tx_id]`) with atomic backups in `.eforge/tx/<tx_id>/` (`manifest.json` + `.bak` payloads) and `blake3` post-write verification.
- **In-Memory Refactoring Simulator (`eforge simulate`)**:
  - Zero-disk-write simulation of file/module moves (`--move <from:to>` or `--plan <yaml>`) predicting relative import rewrites, resolved/introduced Tarjan SCC cycles, and Health Score deltas before touching disk.
- **Temporal Graph Evolution & Architectural Blame (`eforge trend` & `eforge blame-arch`)**:
  - `eforge trend`: Evaluates historical 8-Dimension Health Score trajectories, architectural drift, and composite hotspot rankings (`Churn × Fan-In × Severity`).
  - `eforge blame-arch`: Pinpoints the exact Git commit, author, and timestamp that introduced architectural boundary or circular dependency violations.

### Pillar 3: Intelligent Heuristics (Graph-Theoretic Inference & Closed-Loop Agentic MCP)
- **Topology & Boundary Inference (`eforge rules infer` & `eforge init --infer`)**:
  - Automatically classifies module layers (`UI / Presentation`, `Application / Service`, `Domain / Core`, `Data / Persistence`, `Infrastructure`, `Shared / Utility`) using fan-in, fan-out, and instability metrics $I = \frac{\text{FanOut}}{\text{FanIn} + \text{FanOut}}$, detects project archetype (`Clean / Hexagonal`, `Layered N-Tier`, `Feature-Sliced / Modular`, `Monolithic / Coupled`), and synthesizes custom `forbidden_imports` rules (`--write` to export `.eforge/rules/inferred-architecture.yaml`).
- **Smart Impacted Test Selection (`eforge review --impacted-tests`)**:
  - Maps working-tree or range diffs through reverse transitive module and file import graphs directly to the exact test files and runner command (`vitest`, `cargo test`, `pytest`, `go test`) required to verify a change.
- **4 New Closed-Loop Agentic MCP Tools (`eforge mcp` — 11 Total Tools)**:
  - `eforge_verify_patch`: Pre-flight verification of proposed in-memory file edits against Policy-as-Code and Tarjan SCC before an AI agent writes to disk.
  - `eforge_simulate_refactor`: Zero-disk-write module move simulation over MCP.
  - `eforge_get_impacted_tests`: Deterministic test target selection from a diff or working tree.
  - `eforge_get_pruned_context`: Token-budgeted context packing ranked by graph centrality (`FanIn + FanOut`) within $N$ hops.

---

## [1.0.0] — 2026-10-29 (General Availability Release)

### Summary of v1.0.0 GA Milestone
ELCODE Forge reaches General Availability (`v1.0.0`), delivering a unified, deterministic, local-first code intelligence, static analysis, and enterprise governance platform built under the ELCODE Studio Manifesto:
> *"Software should feel like an instrument, not a compromise."*

---

### Phase 10: Enterprise Governance, Policy-as-Code & Compliance (v1.0.0)
- **Declarative Policy-as-Code Engine (`crates/forge-core`, `crates/forge-rules`, `eforge policy check`)**:
  - Declarative constraints in `eforge.config.yaml`: minimum health score, disallowed severities, zero-tolerance secret enforcement, forbidden licenses (SPDX), circular dependency thresholds, and blast-radius module caps.
  - Time-bounded, path-scoped exception waivers (`expires_at`, `justification`) with automatic expiration checks.
  - Interactive policy evaluation and visual YAML editor in `/governance` dashboard.
- **Automated Compliance Audit Engine (`crates/forge-security`, `crates/forge-cli`, `eforge audit`)**:
  - Automated evaluation against **SOC 2 Type II** (CC6.1, CC6.6, CC7.1, CC8.1).
  - Automated evaluation against **OWASP Top 10 (2021)** (A01, A02, A03, A06).
  - Automated evaluation against **ISO/IEC 27001:2022** (Control 8.8, Control 8.28).
  - Auditable export formats: Terminal summary, GitHub-flavored Markdown, and structured JSON.
- **Multi-Repository Organization Aggregator & Enterprise Auth (`crates/forge-server`)**:
  - Organization portfolio rollups with $O(1)$ fast retrieval via materialized summary view.
  - Cross-Repository Vulnerability Matrix identifying shared vulnerabilities and CVE blast radius across connected workspaces.
  - Enterprise RBAC roles (`Viewer`, `Auditor`, `Maintainer`, `Admin`).
  - JWKS OIDC token verification and reverse-proxy header authentication (`trust_proxy_headers`).
  - Relational append-only immutable audit event stream (`/api/v1/audit-events`).
- **Cryptographic Provenance & SLSA Level 3 (`.github/workflows/release-binaries.yml`)**:
  - SLSA Build Level 3 in-toto provenance generation using `slsa-framework/slsa-github-generator`.
  - GitHub Artifact Attestation with Sigstore Fulcio OIDC signing and Rekor public ledger publication.
  - Exhaustive verification guide in `docs/VERIFICATION_GUIDE.md`.
- **Enterprise Benchmark Suite (`benches/enterprise_benchmarks.rs`)**:
  - Full repository scan latency: < 15s (measured: ~9.5ms on fixtures).
  - Policy evaluation gate latency: < 50ms (measured: ~135µs for 1,000 findings).
  - Memory high-water mark: < 500MB (measured: ~0.06MB working set for 500 AST files).
- **Official Documentation Portal (`apps/docs`)**:
  - Dedicated static documentation site (`pnpm --filter docs run build`) documenting Policy-as-Code, compliance mappings, multi-repo aggregation, and SLSA provenance.

---

### Phase 9: Remote Baseline Synchronization & Collaborative Intelligence (v0.9.0)
- **Forge Server (`crates/forge-server`)**:
  - Self-hostable Axum backend supporting SQLite (`sqlite:`) and PostgreSQL (`postgres://`).
  - API endpoints for baseline ingestion (`POST /api/v1/sync/push`), snapshots (`GET /api/v1/sync/pull`), and branch baseline tracking.
  - Real-time HMAC-SHA256 authenticated webhooks for Slack, Discord, and custom endpoints.
- **CLI Sync Commands (`eforge sync`, `eforge push`, `eforge pull`)**:
  - Local configuration via `eforge.config.yaml` server block or `.eforge/credentials`.
- **Team Comparison UI (`apps/dashboard/src/routes/compare`)**:
  - Visual baseline diffing between working-tree and team server baselines.

---

### Phase 8: Deep Security Intelligence & Supply-Chain Durability (v0.8.0)
- **OSV & GHSA Dependency Vulnerability Scanner (`crates/forge-security`, `eforge security --audit`)**:
  - Live local auditing of dependency manifests across npm, Cargo, PyPI, Go, and Composer against the OSV advisory database.
- **Software Bill of Materials (SBOM) Generator (`eforge sbom`)**:
  - CycloneDX 1.5 JSON and SPDX 2.3 JSON compliant SBOM generation with full PURL metadata.
- **SPDX Copyleft License Safeguard**:
  - Automatic detection of GPL, AGPL, and restrictive copyleft licenses in commercial closed-source projects.

---

### Phase 7: Multi-Language & Polyglot Ecosystem Expansion (v0.7.0)
- Unified AST and import resolution across:
  - **Python** (FastAPI, Flask, Django, intra-package relative imports, stdlib filtering).
  - **Go** (Go modules, intra-module package imports, Gin, Echo, Fiber, Chi).
  - **Rust** (Cargo workspace crates, `mod`, `crate::`, `super::`, Axum, Tokio).
  - **PHP** (Composer, PSR-4 namespace mappings, Laravel, Symfony, WordPress).

---

### Phase 6: Universal Distribution & Packaging (v0.6.0)
- Zero-install execution via `npx elcode-forge`.
- Automated POSIX (`install.sh`) and PowerShell (`install.ps1`) standalone installers.
- Pre-compiled binary distribution via Homebrew tap (`elcode-id/tap/eforge`) and `cargo-binstall`.
- Multi-arch Docker images (`ghcr.io/elcode-id/elcode-forge`).

---

### Phase 5: Deterministic Ecosystem Rules & Shannon Entropy Secret Scanner (v0.5.0)
- 10+ deterministic ecosystem rules for maintainability, architecture, testing, and security.
- Shannon entropy secret scanner detecting high-entropy credentials, tokens, and private keys with zero exfiltration.

---

### Phase 4: Grounded AI PR Review & Native MCP Server (v0.4.0)
- AI provider abstraction (`crates/forge-ai`) supporting Ollama, LM Studio, Anthropic, OpenAI, and Gemini.
- Strictly isolated untrusted data envelopes (`UntrustedEnvelope<T>`) preventing prompt-injection attacks.
- Native Model Context Protocol (MCP) server (`eforge mcp`) exposing deterministic `eforge_*` tools over JSON-RPC 2.0 stdio.

---

### Phase 3: Web Intelligence Dashboard (v0.3.0)
- Local-first SvelteKit web dashboard (`apps/dashboard`) with Cytoscape interactive architecture graphs, file churn hotspot heatmaps, and health score dimension rings.

---

### Phase 2: AST, Import Graph & Tarjan Cycle Detection (v0.2.0)
- Fast regex and tree-based import extraction for TypeScript, JavaScript, Vue, and Svelte.
- Tarjan's Strongly Connected Components algorithm for circular dependency detection.
- Blast-radius change impact calculator traversing reverse module dependency graphs.

---

### Phase 1: Core Foundation & IR (v0.1.0)
- Normalized Intermediate Representation (IR): `Repository`, `FileNode`, `Module`, `Dependency`, `Finding`, `HealthScore`.
- Primary CLI binary `eforge` with alias `elcode-forge`.
- Fast BLAKE3 incremental content hashing and `.eforge/` caching.
