# Multi-Language & Ecosystem Architecture Guide

**Product:** ELCODE Forge (`eforge`)  
**Version:** `v0.7.0`  
**Studio:** [ELCODE](https://elcode.id/) (Bandung, Indonesia)  

ELCODE Forge features a unified, zero-panic, polyglot static analysis engine capable of analyzing heterogeneous codebases, monorepos, and microservices written in **TypeScript**, **JavaScript**, **Python**, **Go**, **Rust**, and **PHP**.

---

## 1. Supported Ecosystems & Capabilities Matrix

| Ecosystem / Language | File Extensions | Manifest & Lockfiles | Framework & Tooling Detectors | Import Resolution Strategy | Dependency Health Analysis |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **TypeScript / JavaScript** | `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`, `.svelte`, `.vue` | `package.json`<br>`package-lock.json`<br>`pnpm-lock.yaml`<br>`yarn.lock`<br>`bun.lockb` | Next.js, Nuxt, SvelteKit, React, Vue, Svelte, Express, NestJS | Relative (`./`, `../`), index resolution, `@/` and `~/` path aliases, SFC `<script>` parsing | Unused dependencies, unpinned/wildcard versions, duplicate declarations |
| **Python** | `.py`, `.pyi` | `pyproject.toml`<br>`requirements.txt`<br>`poetry.lock`<br>`Pipfile.lock` | FastAPI, Flask, Django, Pytest, Celery, SQLAlchemy | Absolute module imports, relative (`from .`, `from ..`), standard library filtering | Unpinned requirements, missing lockfiles, unused packages |
| **Go** | `.go` | `go.mod`<br>`go.sum` | Gin, Echo, Fiber, Chi, Go standard testing | Go module root resolution, intra-module package imports, stdlib filtering | Unpinned/indirect tracking, missing lockfiles (`go.sum`) |
| **Rust** | `.rs` | `Cargo.toml`<br>`Cargo.lock` | Axum, Actix-web, Tokio, Rocket, Cargo test | Workspace crate resolution, inline `mod`, `crate::`, `super::`, `self::`, external crates | Wildcard versions (`*`), unpinned deps, missing `Cargo.lock` |
| **PHP** | `.php` | `composer.json`<br>`composer.lock` | Laravel, Symfony, WordPress, PHPUnit, Pest | PSR-4 namespace mappings (`App\` -> `app/`), single/aliased/group `use` statements, PHP stdlib filtering | Unpinned requirements, missing lockfiles (`composer.lock`), duplicate declarations |

---

## 2. Polyglot Pipeline Architecture

The scanner pipeline executes in single-pass deterministic stages:

```
                          ┌───────────────────────────┐
                          │   scan_repository(path)   │
                          └─────────────┬─────────────┘
                                        │
                                        ▼
                          ┌───────────────────────────┐
                          │  Streaming File Discovery │
                          │ (BLAKE3 Hashing + Filter) │
                          └─────────────┬─────────────┘
                                        │
              ┌─────────────────────────┼─────────────────────────┐
              ▼                         ▼                         ▼
   ┌─────────────────────┐   ┌─────────────────────┐   ┌─────────────────────┐
   │ TypeScript/JS/SFC   │   │   Python Resolver   │   │  Go & Rust Resolvers│
   │   Import Resolver   │   │   (AST + Imports)   │   │ (Packages & Modules)│
   └──────────┬──────────┘   └──────────┬──────────┘   └──────────┬──────────┘
              │                         │                         │
              └─────────────────────────┼─────────────────────────┘
                                        │
                                        ▼
                          ┌───────────────────────────┐
                          │    Unified Architecture   │
                          │        Graph Builder      │
                          └─────────────┬─────────────┘
                                        │
                                        ▼
                          ┌───────────────────────────┐
                          │ Tarjan's SCC Cycle Engine │
                          │ (Deterministic Detection) │
                          └─────────────┬─────────────┘
                                        │
                                        ▼
                          ┌───────────────────────────┐
                          │ Multi-Ecosystem Manifest  │
                          │   Dependency Health Eval  │
                          └─────────────┬─────────────┘
                                        │
                                        ▼
                          ┌───────────────────────────┐
                          │  8-Dimension Health Score │
                          │      (ReportBundle)       │
                          └───────────────────────────┘
```

### 2.1 Namespace Isolation & Collision Guard
In monorepos containing multiple services (e.g. `frontend/src/services` and `backend/app/services`), Forge enforces path-rooted module identification. Every module identifier is relative to the analyzed repository root, preventing module name collisions across distinct languages.

### 2.2 Deterministic Cycle Detection (Tarjan's SCC)
Circular dependencies represent tight coupling and fragile architecture. Forge runs Tarjan's Strongly Connected Components algorithm across all resolved intra-repository import edges. Any strongly connected component with size $> 1$ or self-referential cycle is flagged as `arch/no-circular-dependencies`.

---

## 3. CLI Command Parity Across All Languages

Every `eforge` CLI command supports all six languages natively:

### `eforge doctor`
Diagnoses repository health, languages, frameworks, lockfile hygiene across Node, Python, Go, Rust, and PHP, automated test suites, and secrets in `< 5 seconds`.

### `eforge scan`
Executes full or incremental polyglot scans, computing weighted scores across all 8 health dimensions (*Architecture*, *Security*, *Maintainability*, *Testing*, *Dependencies*, *Documentation*, *Performance*, *CI/CD*).

### `eforge graph`
Renders architecture graphs in Mermaid format (`flowchart TD`) or JSON, with language-aware node clustering.

### `eforge review` & `eforge diff`
Performs blast-radius impact analysis and risk assessment on Git diffs across any combination of Python, Go, Rust, PHP, and TypeScript/JavaScript files.

### `eforge explain [target]`
Explains repository or module architecture, inbound dependents, outbound dependencies, and associated rule findings.

### `eforge dependencies`
Lists all declared and resolved dependencies categorized by ecosystem (`npm`, `PyPI`, `Go`, `Cargo`, `Composer`), flagging unused packages and wildcard versions.

---

## 4. Web Intelligence Dashboard Multi-Ecosystem Experience

The local web dashboard (`apps/dashboard`) provides first-class support for multi-language projects:
- **`/dependencies`**: Interactive filtering pills for `npm`, `PyPI`, `Go`, `Cargo`, and `Composer`, with `DIRECT`, `DEV`, and `INDIRECT` badges.
- **`/findings`**: Dynamic language filter pills and inline location tags.
- **`/architecture`**: Primary language inference per module, table language filtering, and module inspector details.
