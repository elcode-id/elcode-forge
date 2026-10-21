# ELCODE Forge (`eforge`)

> **Know your code. Ship with confidence.**  
> Deterministic Codebase Intelligence, Architecture Analysis, Change Blast-Radius, Release Readiness & Native MCP Engine — built in Rust by **[ELCODE](https://elcode.id/)**.

[![CI — Rust Engine](https://github.com/elcode-id/elcode-forge/actions/workflows/ci-rust.yml/badge.svg)](https://github.com/elcode-id/elcode-forge/actions/workflows/ci-rust.yml)
[![CI — Web, Dashboard & Schemas](https://github.com/elcode-id/elcode-forge/actions/workflows/ci-web.yml/badge.svg)](https://github.com/elcode-id/elcode-forge/actions/workflows/ci-web.yml)
[![CI — Self-Dogfooding](https://github.com/elcode-id/elcode-forge/actions/workflows/eforge-dogfood.yml/badge.svg)](https://github.com/elcode-id/elcode-forge/actions/workflows/eforge-dogfood.yml)
[![CI — Security & Supply-Chain Durability](https://github.com/elcode-id/elcode-forge/actions/workflows/security-durability.yml/badge.svg)](https://github.com/elcode-id/elcode-forge/actions/workflows/security-durability.yml)
[![Release](https://img.shields.io/github/v/release/elcode-id/elcode-forge?label=release)](https://github.com/elcode-id/elcode-forge/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

---

## Why ELCODE Forge?

Modern codebases grow faster than teams can map them—especially with AI-assisted coding. Most developer tools either check syntax at the single-file level (linters) or send unindexed source trees to an LLM and hope for the best.

**ELCODE Forge (`eforge`)** is a **local-first, deterministic codebase intelligence layer**. Before any AI model or code reviewer looks at a repository, `eforge` builds a normalized **Intermediate Representation (IR)** of your repository's files, module boundaries, import graph, circular dependencies, npm/pnpm/Cargo manifest hygiene, Git blast radius, security/secret signals, and an **explainable 8-dimension Health Score**—and exposes it via a high-speed CLI, a zero-telemetry Web Dashboard, and a native **Model Context Protocol (MCP)** server (`eforge mcp`).

### The ELCODE Studio Manifesto (`elcode.id`)

Every architectural boundary, CLI contract, and CI/CD workflow in `elcode-forge` is engineered around the five studio principles of **[ELCODE](https://elcode.id/)**:

| # | Studio Principle | How It Manifests in `elcode-forge` |
| :---: | :--- | :--- |
| **`01`** | **Software should feel like an instrument, not a compromise.** | Collision-free **`eforge`** binary (`elcode-forge` alias), deterministic exit codes (`0..=4`), single-file **`eforge.config.yaml`**, and instant local execution without cloud lock-in or mandatory daemons. |
| **`02`** | **Code quality is visible in product behavior, speed, and durability.** | **Behavior:** Zero-panic Rust engine (`unsafe_code = "forbid"`, zero `.unwrap()`/`.expect()` in library crates).<br>**Speed:** `eforge doctor` completes in **`~18–65ms`** and full workspace scans in **`< 100ms`** using `blake3` content hashing (`.eforge/`).<br>**Durability:** Strict crate DAG, canonical `ReportBundle` JSON Schema drift enforcement, and continuous supply-chain auditing (`cargo audit`). |
| **`03`** | **We use AI as leverage, not as identity.** | **Deterministic before generative.** All AI outputs (`eforge explain --ai`, `eforge ai-review`) are grounded against the deterministic IR via `apply_deterministic_guard`, isolated inside `<untrusted_repository_data>` envelopes, and support offline local models (Ollama / LM Studio). |
| **`04`** | **We prefer precision over decoration.** | Byte-for-byte reproducible JSON, Markdown, and OASIS SARIF v2.1.0 output (`BTreeMap`/`BTreeSet` ordering). Every finding carries explicit `severity`, `confidence`, file/line span, and mathematical 8-dimension health score deductions—zero vague scores or decorative fluff. |
| **`05`** | **We build things worth keeping.** | Self-contained offline dashboard bundles (`eforge report`), append-only historical commit trend tracking (`.eforge/history/`), and automated living onboarding documentation (`eforge onboard`). |

---

## Key Capabilities

| Capability | Description |
| :--- | :--- |
| **Fast Project Diagnosis (`eforge doctor`)** | Diagnoses repository inventory, languages, frameworks (Node/TS/JS, Python, Go, Rust), lockfile hygiene (`package-lock.json`, `pnpm-lock.yaml`, `Cargo.lock`, `go.sum`, `poetry.lock`), test suite presence, and secret exposure in **< 5 seconds** (~4-18ms on small/medium repos). |
| **Declarative Rules & Rule Packs (`eforge rules`)** | Enforces custom architectural, quality, security, and dependency boundaries using 6 deterministic matchers (`forbidden_imports`, `required_companion`, `file_naming`, `file_size`, `regex_pattern`, `dependency_restriction`) and 4 pre-built packs (`recommended`, `strict`, `security`, `clean-architecture`). See [docs/RULES_GUIDE.md](./docs/RULES_GUIDE.md). |
| **Architecture Graph & Cycle Detection (`eforge graph`)** | Infers logical module boundaries across TypeScript, JavaScript, Python, Go, and Rust (including `.vue`/`.svelte` SFCs and path aliases) and detects circular dependencies via **Tarjan's Strongly Connected Components (SCC)** algorithm. |
| **Dependency Health Intelligence (`eforge dependencies`)** | Analyzes multi-ecosystem package manifests across **npm**, **PyPI**, **Go**, and **Cargo** to flag unused production dependencies (`deps/unused-dependency`), wildcard/unpinned versions (`deps/unpinned-version`), and duplicate declarations (`deps/duplicate-dependency`). |
| **Security & Secret Hygiene (`eforge security`)** | Detects hardcoded API keys, tokens, and PEM private keys (`security/hardcoded-secret`, `security/private-key`) with automatic redaction, plus unsafe JS/TS/SFC patterns (`eval`, `new Function`, `dangerouslySetInnerHTML`, `child_process.exec`, permissive CORS). |
| **Git Diff & Blast-Radius Impact (`eforge review`, `eforge diff`)** | Parses unified Git diffs, traverses reverse module dependencies via BFS to separate **directly affected** vs. **transitively affected** modules, flags modified files lacking tests, and computes an explainable **Change Risk Level** (`LOW`, `MEDIUM`, `HIGH`, `CRITICAL`). |
| **Automated Onboarding & Living Docs (`eforge onboard`, `eforge docs`)** | Generates a 5-file deterministic documentation bundle (`architecture.md`, `modules.md`, `dependencies.md`, `onboarding.md`, `risks.md`) with Mermaid `flowchart TD` diagrams in `docs/forge/`. |
| **Project & Module Explainer (`eforge explain [target]`)** | Explains the structural role, files, outgoing imports, incoming dependents, exported symbols, and related findings for the whole repository or any specific module/file path, with optional AI synthesis (`--ai`). |
| **Grounded AI PR Review (`eforge ai-review`)** | Combines deterministic change risk, blast radius, and rule findings with grounded AI insights classified as `Confirmed`, `Probable`, or `Suggestion` under strict deterministic guards. |
| **AI Governance & Privacy (`eforge ai status|usage`)** | Supports local-only AI execution (Ollama, LM Studio), prompt-injection defense envelopes, in-flight secret redaction, daily USD spend ceilings, and SHA-256 caching. |
| **Native MCP Server (`eforge mcp`)** | Exposes 7 deterministic `eforge_*` tools over JSON-RPC 2.0 stdio with `UntrustedEnvelope<T>` isolation, prompt-injection signal scanning, and dynamic rule querying (`eforge_get_rules`). |
| **Explainable 8-Dimension Health Score** | Computes weighted scores (`0..=100`) across *Architecture (20%)*, *Security (20%)*, *Maintainability (15%)*, *Testing (15%)*, *Dependencies (10%)*, *Documentation (10%)*, *Performance (5%)*, and *CI/CD (5%)* with explicit human-readable deduction reasons. |

---

## Multi-Language & Ecosystem Support Matrix

ELCODE Forge provides unified polyglot scanning across 6 core languages with native AST/import analysis, circular dependency detection, and dependency hygiene:

| Ecosystem / Language | File Extensions | Manifest & Lockfiles | Frameworks & Detectors | Import Resolution |
| :--- | :--- | :--- | :--- | :--- |
| **TypeScript / JavaScript** | `.ts`, `.tsx`, `.js`, `.jsx`, `.svelte`, `.vue` | `package.json`, `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lockb` | Next.js, Nuxt, SvelteKit, React, Vue, Svelte, Express, NestJS | Relative, index, `@/` and `~/` aliases, SFC script tags |
| **Python** | `.py`, `.pyi` | `pyproject.toml`, `requirements.txt`, `poetry.lock`, `Pipfile.lock` | FastAPI, Flask, Django, Pytest, Celery, SQLAlchemy | Absolute, intra-package relative (`.`, `..`), stdlib filtering |
| **Go** | `.go` | `go.mod`, `go.sum` | Gin, Echo, Fiber, Chi, Go testing | Go module root resolution, intra-module package imports, stdlib filtering |
| **Rust** | `.rs` | `Cargo.toml`, `Cargo.lock` | Axum, Actix-web, Tokio, Rocket, Cargo test | Workspace crate resolution, inline `mod`, `crate::`, `super::`, `self::` |
| **PHP** | `.php` | `composer.json`, `composer.lock` | Laravel, Symfony, WordPress, PHPUnit, Pest | PSR-4 namespace mappings (`App\` -> `app/`), single/aliased/group `use` statements, stdlib filtering |

For detailed architectural and resolution documentation, see [docs/MULTI_LANGUAGE.md](./docs/MULTI_LANGUAGE.md).

---

## Installation & Distribution

### 1. Zero-Install Execution via npx (Node.js)
No installation required—execute directly in any Node.js / JavaScript / TypeScript project:
```bash
npx elcode-forge doctor
npx elcode-forge scan
npx elcode-forge review --fast
```

### 2. Standalone Installers (Linux, macOS & Windows)
**POSIX (Linux & macOS):**
```bash
curl -fsSL https://raw.githubusercontent.com/elcode-id/elcode-forge/main/scripts/install.sh | bash
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/elcode-id/elcode-forge/main/scripts/install.ps1 | iex
```

### 3. Package Managers & Binary Installers
**Homebrew (macOS & Linux):**
```bash
brew install elcode-id/tap/eforge
```

**Cargo Binstall (Pre-compiled Binary):**
```bash
cargo binstall eforge
```

**Build & Install from Source (Rust Toolchain >= 1.75):**
```bash
cargo install --path crates/forge-cli
```

Verify installation:
```bash
eforge --version
# ELCODE Forge (eforge) v0.7.0
```

---

## Quick Start (The Golden Path)

### 1. Initialize Configuration (`eforge init`)
Generates a starter `eforge.config.yaml` and initializes the `.eforge/` incremental cache directory:
```bash
eforge init --path .
```

### 2. Run a Fast Diagnostic Check (`eforge doctor`)
```bash
eforge doctor --path .
```
```text
ELCODE Forge Doctor
-------------------
[ OK ] Repository inventory: Discovered 71 source/config file(s)
[ OK ] Language & framework detection: Languages: [rust], Frameworks: []
[ OK ] Cargo manifest & lockfile hygiene: Cargo.toml present, Cargo.lock present: true
[ OK ] Automated test suite: Found 26 test file(s)
[ OK ] Security & secret hygiene: No hardcoded secrets or unsafe patterns detected
[ OK ] Architectural & quality rules: Loaded 4 active rule(s) (packs: [recommended], custom: 0)

Summary: 0 warning(s) in 18ms
```

### 3. Declarative Rules Management (`eforge rules`)
```bash
# Initialize a starter custom rules template
eforge rules init

# List active rules and extended packs
eforge rules list

# Check rule syntax and schema without scanning
eforge rules check .eforge/rules/custom-rules.yaml
```

### 4. Execute a Full or Incremental Scan (`eforge scan`)
```bash
# Human-readable terminal report
eforge scan --path .

# Incremental scan using .eforge/metadata.json BLAKE3 hashes
eforge scan --path . --incremental

# Export OASIS SARIF v2.1.0 for GitHub Code Scanning
eforge scan --path . --format sarif --output eforge.sarif

# Enforce CI policy gate (non-zero exit code if High or Critical findings exist)
eforge scan --path . --fail-on high --format github
```

### 5. Focused Architecture, Dependency, Security & Explain Workflows
```bash
# Export Mermaid architecture graph
eforge graph --path . --format markdown

# Inspect npm/pnpm dependency health
eforge dependencies --path . --format terminal

# Run dedicated secret & unsafe pattern security scan
eforge security --path . --format sarif

# Explain a specific module's files, imports, dependents, exports, and findings
eforge explain src/services --path fixtures/next-app --format markdown

# Generate deterministic 5-file onboarding & architecture documentation in docs/forge/
eforge onboard --path .
```

### 6. Review Working-Tree Changes & Blast Radius (`eforge review` & `eforge diff`)
```bash
# Fast diff-scoped review in Markdown format (ideal for PR comments)
eforge review --fast --path . --format markdown

# Inspect direct and transitive module blast radius
eforge diff --path . --format terminal
```

### 7. Native Git Hooks & Staged-Diff Scanner (`eforge hook`)
Install zero-dependency pre-commit and pre-push hooks that review staged changes in `< 200ms`:
```bash
# Install pre-commit and pre-push hooks into .git/hooks/
eforge hook install

# Check installation and backup status
eforge hook status

# Manually trigger pre-commit evaluation against staged files
eforge hook run pre-commit

# Uninstall hooks cleanly (restores pre-existing .forge-backup hooks)
eforge hook uninstall
```

### 8. AI-Assisted Explanations & Grounded PR Review
```bash
# Explain architecture or modules with AI-synthesized, grounded answers
eforge explain src/services --ai

# Preview prompt context pack & token estimates without making network calls
eforge explain src/services --ai --dry-run

# Run grounded AI PR review with Confirmed | Probable | Suggestion items
eforge ai-review --fast --fail-on high

# Check AI provider connectivity and inspect token spend
eforge ai status --ping
eforge ai usage
```

---

## CLI Command Reference (18 Subcommands)

| Command | Key Flags | Description |
| :--- | :--- | :--- |
| `eforge init` | `--path <DIR>`, `--force` | Initializes `eforge.config.yaml` and `.eforge/` cache directory. |
| `eforge doctor` | `--path <DIR>`, `--format <FMT>` | Runs a `< 5s` diagnostic check across inventory, manifest hygiene, tests, rules, and secrets. |
| `eforge rules [list]` | `--path <DIR>`, `--category <CAT>`, `--enabled-only`, `--format <FMT>` | Lists all active rules, categories, severities, and origin packs. |
| `eforge rules check` | `[PATH]`, `--path <DIR>`, `--format <FMT>` | Validates rule syntax against schema with zero panics. |
| `eforge rules init` | `--path <DIR>`, `--force` | Generates starter `.eforge/rules/custom-rules.yaml` template with examples for all 6 matchers. |
| `eforge scan` | `--path <DIR>`, `--incremental`, `--full`, `--fail-on <SEV>`, `--format <FMT>`, `-o <FILE>` | Runs full or incremental architecture, dependency, rule, security, and 8-dimension health analysis. |
| `eforge review` | `--path <DIR>`, `--fast`, `--changed`, `--fail-on <SEV>`, `--format <FMT>` | Computes change risk (`Low|Medium|High|Critical`), blast radius, and findings for modified files. |
| `eforge diff` | `--path <DIR>`, `--format <FMT>` | Parses working-tree unified diff (`git diff HEAD`) and reports direct & transitive module impact. |
| `eforge dependencies` | `--path <DIR>`, `--format <FMT>` | Inspects `package.json` / lockfile dependencies, unused packages, and version hygiene. |
| `eforge security` | `--path <DIR>`, `--format <FMT>` | Runs focused secret detection and unsafe JS/TS/SFC pattern analysis with policy exit code evaluation. |
| `eforge graph` | `--path <DIR>`, `--format <FMT>` | Exports the module architecture graph (`terminal`, `json`, or Mermaid `flowchart TD` in `markdown`). |
| `eforge onboard` | `--path <DIR>`, `-o <OUT_DIR>` | Generates `architecture.md`, `modules.md`, `dependencies.md`, `onboarding.md`, and `risks.md`. |
| `eforge docs` | `--path <DIR>`, `-o <OUT_DIR>` | Alias for `eforge onboard` to regenerate living repository documentation (`docs/forge/`). |
| `eforge explain [target]` | `[TARGET]`, `--path <DIR>`, `--ai`, `--dry-run`, `--format <FMT>` | Explains files, imports, dependents, symbols, and findings; synthesizes answers with `--ai`. |
| `eforge ai-review` | `--base <REF>`, `--fast`, `--fail-on <SEV>`, `--dry-run`, `--no-cache` | Grounded PR review combining blast radius with `Confirmed`, `Probable`, and `Suggestion` items. |
| `eforge ai status` | `--ping`, `--path <DIR>` | Reports AI configuration, selected provider, model, and checks live endpoint latency with `--ping`. |
| `eforge ai usage` | `--path <DIR>`, `--format <FMT>` | Reports daily token consumption, accumulated USD cost, and daily budget ceiling. |
| `eforge report` | `--path <DIR>`, `--output <FILE>`, `--save-history`, `--no-git` | Generates a deterministic `ReportBundle` JSON for dashboard consumption and history snapshots. |
| `eforge serve` | `--port <PORT>`, `--path <DIR>`, `--static-dir <DIR>` | Serves a read-only, loopback-restricted HTTP API (`/api/v1/health`, `/api/v1/bundle`, `/api/v1/history`). |
| `eforge schema` | `--out-dir <DIR>` | Exports canonical JSON Schemas for `ReportBundle` and IR types. |
| `eforge mcp` | `--path <DIR>` | Starts the native Model Context Protocol (MCP) JSON-RPC 2.0 stdio server (`eforge_*` tools). |
| `eforge hook` | `install`, `run <pre-commit|pre-push>`, `uninstall`, `status` | Manages native Git hooks with non-destructive backup preservation and sub-200ms staged-diff scanning. |

**Supported `--format` values:** `terminal` (default), `json`, `markdown`, `sarif` (SARIF v2.1.0), `github` (GitHub Actions annotations + Markdown).

---

## Web Intelligence Dashboard (`apps/dashboard`)

ELCODE Forge provides a local-first, zero-telemetry Web Intelligence Dashboard built with SvelteKit, TypeScript, and Tailwind CSS. The dashboard operates in two complementary modes:

### 1. Static Standalone Mode
Generate a self-contained report bundle and load it offline into the dashboard SPA:
```bash
# Export the deterministic intelligence bundle
eforge report --path . --output report.json

# Open the dashboard SPA and load the JSON bundle
```

### 2. Live Interactive Mode
Run the local read-only daemon for real-time inspection, live auto-updates, and historical commit trends tracking:
```bash
# Save historical commit snapshots as you work
eforge report --save-history

# Start the read-only daemon (strictly bound to 127.0.0.1)
eforge serve --port 7878
```
Connect your dashboard to `http://127.0.0.1:7878` to view:
- **Overview:** 8-dimension health score ring, dimensions breakdown, and repository inventory.
- **Rules Catalog (`/rules`):** Active rule inventory with category/pack filtering, violation counters, and deep links to findings.
- **Architecture:** Interactive module dependency canvas (Cytoscape + Dagre) with circular-dependency highlighting (`#ef4444`) and deep module inspector (`eforge explain` parity). Includes a keyboard-accessible module table as an accessible non-visual alternative.
- **Findings:** URL-synchronized filtering by severity, confidence, rule ID prefix, and file path with hostile plain-text evidence envelopes and rule definition deep links.
- **Security:** Redacted credential leakage detector, unsafe JS/TS pattern analysis, and mitigation steps.
- **Dependencies:** Direct vs dev packages, unpinned version warnings, and unused package flags.
- **Changes & Blast Radius:** Git diff change risk, direct vs transitive module propagation, and churn hotspots.
- **Trends & Evolution:** Historical health score trajectories, finding severity sparklines, and commit snapshot records.

---

## Native MCP Server (`eforge mcp`)

ELCODE Forge includes a built-in, zero-configuration **Model Context Protocol (MCP)** server over JSON-RPC 2.0 stdio (`protocolVersion: "2024-11-05"`), enabling AI coding assistants (Claude Code, Claude Desktop, Cursor, Windsurf, Google Antigravity) to query deterministic codebase intelligence without hallucinating architecture boundaries.

### MCP Client Configuration
Add `eforge` to your MCP client configuration (`mcp_config.json` / `.cursor/mcp.json` / `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "elcode-forge": {
      "command": "eforge",
      "args": ["mcp", "--path", "/absolute/path/to/your/repo"]
    }
  }
}
```

### Exposed `eforge_*` Tools

| Tool Name | Arguments | Description |
| :--- | :--- | :--- |
| `eforge_project_info` | `{}` | Returns repository profile, languages, frameworks, package managers, inventory counts, and 8-dimension health score. |
| `eforge_get_architecture` | `{}` | Returns the module architecture graph, inter-module imports, and detected circular dependency cycles. |
| `eforge_get_dependencies` | `{}` | Returns declared/locked dependencies and `deps/*` hygiene findings. |
| `eforge_get_findings` | `{ "severity"?: string, "rule_prefix"?: string }` | Returns sorted architectural, quality, dependency, and security findings with optional filters. |
| `eforge_get_change_impact` | `{ "diff_text"?: string }` | Computes direct/transitive module blast radius and change risk score for a unified diff or working tree. |
| `eforge_explain_module` | `{ "target": string }` | Explains matched modules, files, inbound dependents, outbound dependencies, and findings for `target`. |
| `eforge_get_rules` | `{ "category"?: string, "enabled_only"?: boolean }` | Returns active rule definitions, severity, description, remediation, and source pack. |

### Prompt-Injection Defense (`UntrustedEnvelope<T>`)
Every MCP tool response is wrapped in a deterministic `UntrustedEnvelope<T>` (`trust_level: "untrusted_repository_data"`) that instructs the host LLM to treat repository text strictly as passive data and scans for prompt-injection signals (`override_previous_instructions`, `disregard_instructions`, `system_prompt_reference`, `role_hijack_attempt`, `chatml_delimiter_injection`, `command_execution_directive`, `exfiltration_directive`).

---

## GitHub Action (`elcode-id/forge-action`)

Use the composite GitHub Action in [`integrations/github-action`](./integrations/github-action/README.md) to gate pull requests and publish Markdown summaries + SARIF reports:

```yaml
- name: Run ELCODE Forge PR Review
  uses: ./integrations/github-action
  with:
    command: review
    path: .
    strict: "true"
    step-summary: "true"
```

---

## AI Architecture, Privacy & Cost Controls

ELCODE Forge adheres to a strict principle: **Generative AI is an advisory layer, never an authoritative judge.**

- **Local-First & Offline Support:** Native `openai-compatible` provider support for local models via **Ollama** (`http://localhost:11434/v1`) or **LM Studio** (`http://localhost:1234/v1`). Zero code leaves your workstation.
- **Untrusted Envelopes & Prompt Injection Defense:** All repository code, AST snippets, and git diffs are strictly wrapped in `<untrusted_repository_data>` envelopes.
- **Secret Redaction:** High-entropy credentials, private keys, and API tokens are detected and redacted (`[REDACTED_SECRET]`) prior to prompt construction.
- **Deterministic Grounding Guard:** The `apply_deterministic_guard` engine cross-verifies all AI review items against deterministic scan findings. `Confirmed` items require verifiable finding IDs, and hallucinated paths are dropped.
- **Cost Controls:** Pre-call context inspection (`--dry-run`), daily USD expenditure ceilings (`budget_daily_usd`), and deterministic SHA-256 prompt caching (`.eforge/ai-cache/`).

For full details, read [docs/AI_AND_PRIVACY.md](./docs/AI_AND_PRIVACY.md).

---

## Configuration (`eforge.config.yaml`)

ELCODE Forge reads **`eforge.config.yaml`** from the repository root (with automatic backward-compatible fallback to `forge.config.yaml`). If no config file is present, sensible defaults are used automatically.

```yaml
# ELCODE Forge (eforge) Configuration
version: 1

project_name: "my-web-platform"
languages:
  - "typescript"
  - "javascript"

ignore_patterns:
  - "node_modules/**"
  - "dist/**"
  - "build/**"
  - ".next/**"
  - "coverage/**"
  - ".eforge/**"

git_enabled: true
fail_on_severity: "high"

# Rules hierarchy & pack extensions
rules:
  extends:
    - recommended
    - clean-architecture
  custom_packs:
    - .eforge/rules/custom-arch.yaml

  # Optional per-rule overrides
  quality/max-file-lines:
    enabled: true
    severity: "low"
    max_lines: 400

# Optional AI Assistant & Review Configuration
# ai:
#   enabled: false
#   provider: "openai"              # openai | anthropic | gemini | openai-compatible | mock
#   model: "gpt-4o"
#   # base_url: "http://localhost:11434/v1" # Local Ollama / LM Studio
#   budget_daily_usd: 2.00
```

---

## Deterministic Exit Codes (`0..=4`)

Designed for predictable CI/CD gates:

| Exit Code | Enum Variant | Meaning |
| :---: | :--- | :--- |
| `0` | `ExitCode::Success` | Execution succeeded; no findings exceeded the configured `--fail-on` threshold. |
| `1` | `ExitCode::FindingsAboveThreshold` | Architectural, quality, or dependency findings met or exceeded the threshold. |
| `2` | `ExitCode::ConfigurationError` | Invalid CLI arguments or malformed `eforge.config.yaml`. |
| `3` | `ExitCode::ExecutionError` | Unexpected I/O, filesystem, or runtime error. |
| `4` | `ExitCode::SecurityPolicyFailure` | Security or hardcoded-secret finding met or exceeded the threshold. |

---

## Workspace Architecture

`elcode-forge` is organized as a modular Rust workspace with a strict Directed Acyclic Graph (DAG) dependency boundary:

```mermaid
flowchart TD
    CLI["crates/forge-cli (eforge / elcode-forge)"] --> ANALYZER["crates/forge-analyzer"]
    CLI --> GIT["crates/forge-git"]
    CLI --> RULES["crates/forge-rules"]
    CLI --> SECURITY["crates/forge-security"]
    CLI --> AI["crates/forge-ai"]
    CLI --> MCP["crates/forge-mcp"]
    CLI --> CORE["crates/forge-core"]

    MCP --> ANALYZER
    MCP --> GIT
    MCP --> RULES
    MCP --> SECURITY
    MCP --> CORE
    AI --> CORE
    ANALYZER --> CORE
    GIT --> CORE
    RULES --> CORE
    SECURITY --> CORE
```

- **`crates/forge-core`** — Leaf crate: Normalized IR (`Repository`, `FileNode`, `Module`, `Dependency`, `Finding`, `HealthScore`), `ForgeConfig` & `RuleOverrideConfig` loader, canonical `path_guard`, `.eforge/` BLAKE3 cache, and 8-dimension scoring engine.
- **`crates/forge-analyzer`** — Gitignore-aware repository scanner, language/framework detector, `package.json` / `package-lock.json` / `pnpm-lock.yaml` dependency analyzer, and TS/JS + `.vue`/`.svelte` SFC import graph builder with Tarjan SCC cycle detection.
- **`crates/forge-git`** — Unified diff parser, commit hotspot analyzer, and reverse-import BFS blast-radius calculator.
- **`crates/forge-rules`** — Declarative custom rules engine, 6 deterministic matchers, 4 pre-built rule packs (`recommended`, `strict`, `security`, `clean-architecture`), and config extends hierarchy.
- **`crates/forge-security`** — Redacted secret scanner and unsafe JS/TS/SFC pattern detector.
- **`crates/forge-ai`** — Provider abstraction (`openai`, `anthropic`, `gemini`, `openai-compatible`), context pack builder, secret redaction, token budgeting, and response caching.
- **`crates/forge-mcp`** — Native Model Context Protocol (MCP) JSON-RPC 2.0 stdio server (`eforge_*` tool namespace) and `UntrustedEnvelope<T>` prompt-injection boundary.
- **`crates/forge-cli`** — CLI entrypoint (`eforge` & `elcode-forge`), 18 subcommands, and report formatters (`terminal`, `json`, `markdown`, `sarif`, `github`).
- **`integrations/github-action`** — Composite GitHub Action (`elcode-id/forge-action`).
- **`fixtures/`** — Curated deterministic test repositories (`fixtures/next-app`, `fixtures/vulnerable-ts-app`, `fixtures/ai-eval`, `fixtures/custom-rules-app`).

---

## CI/CD & GitFlow Quality Gate Discipline

In alignment with ELCODE Principle `02` (*"Code quality is visible in product behavior, speed, and durability."*) and Principle `04` (*"We prefer precision over decoration."*), every change follows a strict **GitFlow (`feat/* -> develop -> main`)** and **100% Green-CI Pre-Merge Gate**:

### Automated GitHub Actions Workflows (`.github/workflows/`)

| Workflow | File | Enforcement Gate |
| :--- | :--- | :--- |
| **CI — Rust Engine** | [`.github/workflows/ci-rust.yml`](./.github/workflows/ci-rust.yml) | Path-aware selective verification (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` + CLI smoke test on `fixtures/`). |
| **CI — Web, Dashboard & Schemas** | [`.github/workflows/ci-web.yml`](./.github/workflows/ci-web.yml) | Canonical JSON Schema drift check (`eforge schema` + `git diff --exit-code`), TypeScript strict check, ESLint, Vitest, and SvelteKit static build. |
| **CI — Self-Dogfooding (`eforge`)** | [`.github/workflows/eforge-dogfood.yml`](./.github/workflows/eforge-dogfood.yml) | Runs `eforge doctor`, `eforge scan --fail-on high`, `eforge review --fast`, `eforge explain --ai --dry-run`, `eforge ai-review --dry-run`, and `eforge report` against this repository itself. |
| **CI — Security & Supply-Chain Durability** | [`.github/workflows/security-durability.yml`](./.github/workflows/security-durability.yml) | Runs `cargo audit` against the RustSec Advisory Database on `Cargo.lock` and executes `eforge security --format sarif`. |
| **Release — Cross-Platform Binaries** | [`.github/workflows/release-binaries.yml`](./.github/workflows/release-binaries.yml) | Builds `--locked` release binaries (`eforge` & `elcode-forge`) for Linux (`x86_64`), macOS (`aarch64`), and Windows (`x86_64`) with `SHA256SUMS.txt` verification. |

### Mandatory Pre-Merge Protocol

No Pull Request may be squash-merged into `develop` or `main` until both local verification and live GitHub Actions checks pass with exit code `0`:

```bash
# 1. Local Rust Engine Gate
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib --bins

# 2. Local Web & Schema Drift Gate
cargo run --bin eforge -- schema --out-dir packages/forge-schema/schemas
git diff --exit-code packages/forge-schema/schemas
pnpm run lint
pnpm run check
pnpm run test

# 3. Live GitHub Actions Green-CI Gate (Mandatory before squash-merge)
gh pr checks <PR_NUMBER> --watch
gh pr merge <PR_NUMBER> --squash --delete-branch
```

---

## License

Released under the [MIT License](./LICENSE).  
Built with precision by **[ELCODE](https://elcode.id/)** (Bandung, Indonesia).
