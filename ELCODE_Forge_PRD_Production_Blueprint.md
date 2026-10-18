# ELCODE FORGE — Product Requirements Document

**Codename:** Forge  
**Product:** ELCODE Forge  
**Tagline:** **Know your code. Ship with confidence.**  
**Organization:** ELCODE  
**Owner:** Luki Rahayu Agung / ELCODE  
**Document Status:** Production Blueprint  
**Version:** 1.1  
**Date:** 2026-10-18  
**License Direction:** Open Source Core + permissive license, with future commercial extensions

---

## 1. Executive Summary

ELCODE Forge is an open-source **codebase intelligence and engineering workflow platform** designed to help developers understand, validate, review, and continuously improve modern software projects.

Forge is intentionally **not another AI coding agent**.

Its core purpose is to answer questions that become increasingly important as software is produced faster with AI:

- What is inside this codebase?
- How is the architecture actually connected?
- Where are the risks?
- What changed and what could break?
- Is the project maintainable?
- Are security and engineering standards being followed?
- Can a new developer understand the project quickly?
- Can AI agents safely operate within the project's actual architecture and conventions?

Forge combines deterministic analysis, static analysis, repository intelligence, configurable rules, Git/GitHub integration, optional AI capabilities, and an MCP interface.

The long-term vision is to become a **developer-facing intelligence layer for software projects**.

---

# 2. Product Vision

> **ELCODE Forge makes every codebase understandable, measurable, and safer to change.**

The product should eventually provide a continuous intelligence loop:

```text
                    ELCODE FORGE
                         │
              ┌──────────┴──────────┐
              │                     │
        UNDERSTAND               VALIDATE
              │                     │
      Architecture             Security
      Dependencies             Quality
      Structure                 Testing
      Ownership                 Risk
              │                     │
              └──────────┬──────────┘
                         │
                       IMPROVE
                         │
              ┌──────────┼──────────┐
              │          │          │
            Human       AI        Automation
              │          │          │
              └──────────┼──────────┘
                         │
                  SHIP WITH CONFIDENCE
```

---

# 3. Strategic Positioning

## 3.1 Category

**Codebase Intelligence & Engineering Quality Platform**

Forge sits between:

- static analysis tools,
- code quality tools,
- dependency scanners,
- architecture documentation tools,
- GitHub review automation,
- AI coding assistants,
- developer onboarding tools.

It does not attempt to replace all of them.

Instead, it creates an intelligence layer that connects their signals around the **actual repository**.

## 3.2 Positioning Statement

> For developers and engineering teams working on modern codebases, ELCODE Forge is an open-source codebase intelligence platform that explains architecture, detects engineering risks, reviews changes, and provides AI-ready project context.

## 3.3 Differentiation

Forge should differentiate around:

1. **Repository-first intelligence**
2. **Architecture awareness**
3. **AI-assisted but AI-optional workflows**
4. **Developer onboarding**
5. **Change-risk analysis**
6. **GitHub-native workflow**
7. **MCP-native access for AI agents**
8. **Extensible rules and analyzers**
9. **Local-first execution**
10. **Open-source transparency**

---

# 4. Why Forge Should Exist

AI-assisted development increases the volume and speed of code changes.

That creates a second-order problem:

> Code can be produced faster than humans can understand, review, and maintain it.

Forge addresses the resulting gap:

```text
AI / Human
    │
    ▼
Code Changes
    │
    ▼
ELCODE Forge
    │
    ├── Understand
    ├── Analyze
    ├── Validate
    ├── Explain
    └── Review
    │
    ▼
Safer Changes
```

The product should therefore avoid competing directly on "who writes code fastest".

It should compete on:

> **Who helps developers understand and ship changes with the least risk?**

---

# 5. Target Users

## 5.1 Primary Users

### A. Solo Developers

Needs:

- fast project health checks,
- dependency visibility,
- architecture understanding,
- security checks,
- AI-assisted explanations.

### B. Professional Developers

Needs:

- change impact analysis,
- code review,
- architecture visualization,
- onboarding,
- custom rules.

### C. Open Source Maintainers

Needs:

- PR quality checks,
- contributor onboarding,
- architecture documentation,
- automated reports.

### D. Small Engineering Teams

Needs:

- consistent engineering standards,
- CI checks,
- project health,
- team onboarding.

## 5.2 Secondary Users

### E. Software Agencies / Studios

Useful for:

- project audits,
- handovers,
- client reporting,
- technical debt visibility,
- maintenance contracts.

### F. Technical Consultants

Useful for:

- architecture assessment,
- security review,
- legacy system analysis,
- technical due diligence.

### G. Engineering Managers / Tech Leads

Useful for:

- engineering health metrics,
- trend monitoring,
- technical debt,
- PR risk.

### H. AI-Agent Developers

Useful for:

- repository context,
- MCP tools,
- architecture queries,
- safe change validation.

---

# 6. Jobs To Be Done

## JTBD 1 — Understand a New Codebase

> "I inherited this repository. Show me how it works."

Forge should produce:

- architecture overview,
- framework/runtime,
- entry points,
- important modules,
- dependency graph,
- database/API relationships,
- development commands,
- onboarding guide.

## JTBD 2 — Assess Project Health

> "How healthy is this codebase?"

Forge should calculate a transparent health model.

## JTBD 3 — Review Changes

> "What risk does this PR introduce?"

Forge should analyze:

- changed files,
- dependency changes,
- architecture impact,
- security impact,
- test coverage signals,
- complexity changes.

## JTBD 4 — Prepare AI Context

> "Give my coding agent an accurate understanding of this project."

Forge should expose structured repository knowledge through MCP.

## JTBD 5 — Maintain Standards

> "Make sure every project follows our engineering rules."

Forge should support custom rules and policies.

---

# 7. Product Principles

1. **Local-first**
2. **AI optional**
3. **Deterministic before generative**
4. **Explain every score**
5. **Never hide uncertainty**
6. **Fast feedback over exhaustive pipelines**
7. **Git-native**
8. **Configuration as code**
9. **Composable architecture**
10. **Open-source by default**
11. **Security before convenience**
12. **Progressive complexity**

---

# 8. Product Surface

Forge consists of five major surfaces:

```text
ELCODE Forge
│
├── Forge CLI
├── Forge Core Engine
├── Forge GitHub Integration
├── Forge Dashboard
└── Forge MCP Server
```

## 8.1 CLI

Primary binary command:

```bash
eforge
```

*(With optional secondary alias `elcode-forge` provided via package distributions. Using `eforge` eliminates `PATH` collisions with Foundry `forge`, Laravel Forge CLI, and Atlassian Forge CLI while reinforcing the ELCODE brand.)*

Examples:

```bash
eforge init
eforge scan
eforge doctor
eforge explain
eforge review
eforge ai-review
eforge security
eforge dependencies
eforge docs
eforge graph
eforge report
eforge diff
eforge onboard
eforge rules
eforge mcp
eforge config
```

## 8.2 Core Engine

Responsible for:

- repository discovery,
- parsing,
- AST analysis,
- dependency analysis,
- Git analysis,
- architecture inference,
- rules,
- scoring,
- report generation.

## 8.3 GitHub Integration

Provides:

- GitHub Action,
- PR review,
- annotations,
- checks,
- comments,
- status reporting,
- optional GitHub App.

## 8.4 Dashboard

Provides:

- project overview,
- health score,
- architecture graph,
- findings,
- trends,
- PR history,
- dependency health,
- security findings.

## 8.5 MCP Server

Provides repository intelligence to compatible AI agents.

---

# 9. Core Functional Requirements

## 9.1 Project Discovery

Forge MUST detect:

- programming languages,
- frameworks,
- runtimes,
- package managers,
- build systems,
- test frameworks,
- databases when detectable,
- Docker configuration,
- CI/CD configuration,
- infrastructure configuration.

Supported initial ecosystems should include:

### JavaScript / TypeScript

- Node.js
- npm
- pnpm
- yarn
- Bun
- React
- Next.js
- Vue
- Nuxt
- Svelte
- SvelteKit
- Express
- NestJS

### Python

- Python
- FastAPI
- Flask
- Django
- Poetry
- uv
- pip

### Other strategic ecosystems

- PHP
- Laravel
- Go
- Rust

Additional ecosystems should be plugin-based.

---

# 10. Codebase Intelligence Engine

## 10.1 Repository Scanner

The scanner builds a normalized repository model:

```text
Repository
├── Metadata
├── Files
├── Modules
├── Symbols
├── Dependencies
├── Imports
├── Exports
├── Tests
├── Configurations
├── Git history
└── Infrastructure
```

## 10.2 AST Analysis

Where possible, Forge SHOULD use language-native or high-performance parsers.

The architecture should support:

```text
Parser
   ↓
Language-neutral Intermediate Representation
   ↓
Analyzer
```

This allows different analyzers to work across languages.

## 10.3 Dependency Analysis

Detect:

- direct dependencies,
- transitive dependencies,
- unused dependencies,
- duplicate dependencies,
- outdated dependencies,
- dependency concentration,
- risky packages.

Forge should integrate external vulnerability databases where appropriate rather than maintaining its own vulnerability database.

---

# 11. Architecture Intelligence

Forge should infer:

- module boundaries,
- import relationships,
- dependency direction,
- circular dependencies,
- layered architecture,
- high-centrality modules,
- isolated modules,
- potential architectural violations.

Example:

```text
Pages
  ↓
Components
  ↓
Composables
  ↓
Services
  ↓
API
  ↓
Database
```

Potential violation:

```text
Component ───────► Database
```

Forge should report:

> Component directly accesses database layer, bypassing service boundary.

Architecture inference MUST clearly distinguish:

- detected facts,
- inferred relationships,
- heuristic recommendations.

---

# 12. Architecture Graph

The dashboard should provide an interactive graph.

Capabilities:

- zoom,
- filter,
- search,
- module grouping,
- dependency direction,
- circular dependency highlighting,
- impact visualization.

Graph levels:

1. Repository
2. Module
3. File
4. Symbol

The default view should avoid overwhelming users.

---

# 13. Codebase Health Score

Forge should calculate a transparent score.

Suggested dimensions:

```text
Architecture        20%
Security             20%
Maintainability      15%
Testing              15%
Dependencies         10%
Documentation        10%
Performance          5%
CI/CD                5%
```

However, scores MUST remain configurable.

Example:

```text
Forge Health
─────────────
87 / 100

Architecture     92
Security         88
Maintainability  84
Testing          71
Dependencies     94
Documentation    63
Performance      91
CI/CD            95
```

## Critical Principle

Forge MUST NOT present a score as absolute truth.

Every score must be explainable:

```text
Testing: 71

Reasons:
- 82% testable modules detected
- 61% estimated coverage signal
- 14 changed files without related tests
- 3 critical services have no detected tests
```

---

# 14. Finding Model

Every finding should use a normalized schema:

```yaml
id:
category:
severity:
confidence:
title:
description:
evidence:
location:
recommendation:
rule:
documentation:
autofix:
```

Severity:

```text
critical
high
medium
low
info
```

Confidence:

```text
certain
high
medium
low
```

This distinction is important because heuristic analysis can be uncertain.

---

# 15. Security Analysis

Forge should provide a security analyzer.

Initial checks:

- committed secrets,
- suspicious environment exposure,
- unsafe shell execution,
- insecure dependency patterns,
- dangerous deserialization patterns,
- unsafe filesystem operations,
- suspicious eval usage,
- CORS risks,
- security header signals,
- iframe/security configuration,
- authentication boundary issues,
- dependency vulnerabilities.

Forge should integrate established security tooling rather than reinvent every scanner.

Examples of integrations may include:

- Semgrep,
- OSV,
- Trivy,
- GitHub security APIs.

Forge's value is aggregation, context, prioritization, and project-aware interpretation.

---

# 16. Dependency Intelligence

Provide:

```bash
eforge dependencies
```

Output:

```text
Dependency Health

Critical vulnerabilities     0
High vulnerabilities         2
Outdated packages            7
Unused packages              3
Duplicate versions           4
```

Additional features:

- dependency age,
- update risk,
- breaking version detection,
- license metadata,
- dependency graph,
- transitive impact.

---

# 17. Git Intelligence

Forge should understand Git history.

Analyze:

- change frequency,
- hotspots,
- ownership,
- churn,
- files frequently changed together,
- bug-prone areas when reliable signals exist,
- abandoned modules,
- recent architecture changes.

Potential report:

```text
Change Hotspots

auth/
billing/
reporting/

auth/session.ts

Changes: 43
Contributors: 5
Recent incidents: 3
Dependency centrality: high

Risk: HIGH
```

Do not infer causal relationships without evidence.

---

# 18. Change Impact Analysis

Command:

```bash
eforge diff
```

or:

```bash
eforge review
```

Given a Git diff, Forge should identify:

```text
Changed:
UserService

Potentially affected:
AuthController
UserProfile
AdminDashboard
SessionStore
```

Output:

```text
CHANGE IMPACT

Risk: Medium

Direct impact:
- UserService
- UserRepository

Potential impact:
- Auth
- Admin
- User profile

Missing validation:
- UserService integration test
```

---

# 19. AI Layer

AI is an optional layer.

Forge MUST remain useful without AI.

Supported architecture:

```text
AI Provider
├── OpenAI
├── Anthropic
├── Gemini
├── OpenRouter
├── Ollama
└── Local compatible endpoints
```

Provider interface:

```text
generate()
stream()
embed()
structured_output()
```

The provider layer must be replaceable.

---

# 20. AI Codebase Explainer

Commands:

```bash
eforge explain
eforge explain auth
eforge explain src/modules/payment
```

AI should receive structured context from Forge rather than blindly receiving the entire repository.

Context pipeline:

```text
Repository
   ↓
Forge Index
   ↓
Relevant Context
   ↓
Policy / Rules
   ↓
LLM
   ↓
Structured Answer
```

This reduces:

- token waste,
- hallucination,
- privacy exposure,
- latency.

---

# 21. AI Change Review

Command:

```bash
eforge ai-review
```

AI should review only after deterministic analysis.

Pipeline:

```text
Git Diff
   ↓
Deterministic Analysis
   ↓
Architecture Context
   ↓
Security Findings
   ↓
Tests
   ↓
AI Interpretation
```

AI output must distinguish:

- confirmed issue,
- probable issue,
- suggestion.

AI MUST NOT override deterministic security findings.

---

# 22. MCP Server

Forge should expose repository intelligence through MCP using the `eforge_*` tool namespace to prevent collisions with other MCP servers in client configurations (such as Cursor and Claude Desktop).

Example tools:

```text
eforge_project_info
eforge_search_code
eforge_get_architecture
eforge_get_dependencies
eforge_get_findings
eforge_get_file_context
eforge_get_change_impact
eforge_explain_module
eforge_get_rules
```

Example AI-agent interaction:

```text
Agent:
"What depends on AuthService?"

Forge:
"12 modules depend directly or indirectly on AuthService.
Highest-risk consumers:
1. SessionManager
2. AdminController
3. UserProfile"
```

MCP must enforce repository boundaries and permissions.

---

# 23. Developer Onboarding

Command:

```bash
eforge onboard
```

Should generate:

```text
docs/forge/
├── architecture.md
├── modules.md
├── dependencies.md
├── development.md
├── onboarding.md
└── risks.md
```

The onboarding report should answer:

- What is this project?
- How do I run it?
- Where does the application start?
- Where is authentication?
- Where are APIs?
- Where is persistence?
- Where are tests?
- How is it deployed?
- What are the major risks?

---

# 24. Documentation Intelligence

Command:

```bash
eforge docs
```

Possible outputs:

- architecture documentation,
- module documentation,
- API map,
- dependency documentation,
- onboarding documentation,
- ADR suggestions.

Generated documentation MUST be clearly marked as generated.

---

# 25. `eforge doctor`

The doctor command is intended as the fastest entry point.

```bash
eforge doctor
```

Checks:

- runtime,
- package manager,
- dependency health,
- Git state,
- environment configuration,
- test configuration,
- CI configuration,
- Docker,
- security basics,
- Forge configuration.

Example:

```text
ELCODE FORGE DOCTOR

✓ Repository detected
✓ Git detected
✓ TypeScript detected
✓ pnpm detected
⚠ 3 vulnerable dependencies
⚠ 4 services without tests
✓ GitHub Actions detected

Result: 2 warnings
```

Target execution time for common projects: **<5 seconds** where feasible.

---

# 26. Rule Engine

Forge must provide a first-class rules system.

Example:

```yaml
rules:
  no-direct-api-in-components:
    severity: warning

  require-tests-for-services:
    severity: error

  max-component-lines:
    threshold: 300
```

Rules should support:

- built-in rules,
- project rules,
- organization rules,
- language-specific rules,
- framework-specific rules,
- custom analyzers.

---

# 27. Rule Packs

Future ecosystem:

```text
@elcode/forge-rules-web
@elcode/forge-rules-next
@elcode/forge-rules-nuxt
@elcode/forge-rules-security
@elcode/forge-rules-node
```

A rule pack should be installable and versioned.

---

# 28. Configuration

Default:

```text
eforge.config.yaml
```

*(With backward-compatible fallback support for reading `forge.config.yaml`.)*

Example:

```yaml
version: 1

project:
  name: my-app

analysis:
  languages:
    - typescript
    - javascript

rules:
  extends:
    - recommended
    - security

ai:
  enabled: false

git:
  enabled: true

ignore:
  - node_modules
  - dist
  - .next
```

Configuration must support deterministic CI execution.

---

# 29. CLI UX

The CLI should feel fast and modern.

Example:

```bash
eforge scan
```

Output:

```text
Scanning repository...

✓ Repository
✓ Languages
✓ Dependencies
✓ Architecture
✓ Git
✓ Security

Analysis complete in 3.4s

Health: 87/100
Findings: 12
```

Avoid excessive animations.

Provide machine-readable output:

```bash
eforge scan --format json
eforge scan --format sarif
eforge scan --format markdown
eforge scan --format github
```

---

# 30. GitHub Integration

## GitHub Action

Example:

```yaml
name: Forge

on:
  pull_request:

jobs:
  forge:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write

    steps:
      - uses: actions/checkout@v4

      - uses: elcode-id/forge-action@v1
```

## PR Result

```text
ELCODE FORGE

Risk: MEDIUM

Architecture     ✓
Security         ⚠
Testing          ⚠
Dependencies     ✓

3 findings

[View Forge Report]
```

---

# 31. SARIF Support

Forge should support SARIF output for compatibility with security and code scanning workflows.

```bash
eforge scan --format sarif > eforge.sarif
```

This enables integration with existing GitHub security workflows.

---

# 32. Dashboard

The dashboard should not become a generic project-management application.

Its focus is engineering intelligence.

Primary screens:

1. Overview
2. Architecture
3. Findings
4. Dependencies
5. Changes
6. Security
7. Rules
8. Reports
9. AI
10. Settings

## Overview

```text
Project Health
87 / 100

Architecture     92
Security         88
Testing          71
Dependencies     94
Documentation    63

Recent changes
Risk trend
Top hotspots
Critical findings
```

---

# 33. Dashboard Technology

Recommended:

- **SvelteKit + TypeScript + Tailwind CSS + `@elcode/design-system`** (or **Next.js + React + TypeScript** if separated for ecosystem compatibility)
- accessible component system
- graph visualization library
- API layer compatible with Forge server

> **Architectural Alignment Note:** Regardless of whether the Dashboard uses SvelteKit (`@elcode/design-system`) or Next.js, the **Rust Core Engine MUST remain 100% UI-agnostic**, exposing all repository intelligence via deterministic schemas, CLI outputs, and APIs.

The UI should follow ELCODE's premium technical identity:

- restrained,
- modern,
- developer-oriented,
- high information density,
- strong typography,
- minimal decoration.

---

# 34. Backend / Server Architecture

The cloud/server component should be optional.

Possible architecture:

```text
                 Forge CLI
                    │
             Local Forge Core
                    │
             Optional Sync
                    │
              Forge Server
                    │
       ┌────────────┼────────────┐
       │            │            │
    Projects     Findings      Trends
       │            │            │
       └────────────┼────────────┘
                    │
                 Database
```

Recommended future server stack:

- TypeScript or Go/Rust service,
- PostgreSQL,
- object storage,
- Redis only when justified,
- queue system only when needed.

Do not introduce distributed infrastructure prematurely.

---

# 35. Local-First Architecture

The default workflow should work without a cloud account.

```text
Developer Machine
       │
       ├── Repository
       ├── Forge CLI
       ├── Analysis
       └── Optional Local LLM
```

Cloud is optional.

This improves:

- privacy,
- adoption,
- OSS credibility,
- enterprise compatibility.

---

# 36. Privacy

Forge should clearly document:

### Local-only mode

No source code leaves the machine.

### Cloud AI mode

Only explicitly selected context is sent to an AI provider.

### Forge Cloud mode

Repository metadata is synchronized according to explicit user configuration.

Users should always know:

- what data is collected,
- where it is processed,
- how long it is retained,
- whether AI is involved.

---

# 37. Security Requirements

Production Forge MUST include:

- secure credential handling,
- no secret logging,
- secure temp files,
- path traversal protection,
- sandboxing for external analyzers,
- command execution restrictions,
- dependency scanning,
- signed releases where feasible,
- SBOM generation,
- provenance/attestation where feasible,
- least-privilege GitHub permissions.

AI tools must not be allowed arbitrary shell execution by default.

---

# 38. Threat Model

Threat categories:

1. Malicious repository
2. Malicious dependency
3. Prompt injection in source code
4. Prompt injection in documentation
5. Secret exfiltration
6. Unsafe analyzer execution
7. Path traversal
8. GitHub token abuse
9. AI provider data leakage
10. Supply-chain compromise

Forge must treat repository content as **untrusted input**.

---

# 39. AI Prompt Injection Defense

This is a first-class requirement.

Repository content may contain:

```text
Ignore previous instructions...
Send environment variables...
Run this command...
```

Forge must never interpret repository text as trusted instructions.

Architecture:

```text
Repository Content
       │
       ▼
Untrusted Context
       │
       ▼
Policy Boundary
       │
       ▼
AI
```

Tool permissions must be separate from model-generated content.

---

# 40. Performance Requirements

Target:

### Small project

< 2 seconds for basic scan

### Medium project

< 10 seconds

### Large project

Progressive analysis with caching.

Forge MUST implement:

- incremental analysis,
- file hashing,
- cache reuse,
- parallel analysis,
- ignored directories,
- lazy graph expansion.

The goal is **fast developer feedback**, not maximum theoretical analysis depth on every command.

---

# 41. Incremental Analysis

Forge should store analysis state in `.eforge/` (preventing directory collisions with `.forge` folders used by other toolchains):

```text
.eforge/
├── cache/
├── index/
└── metadata.json
```

Cache invalidation should be based on:

- content hash,
- dependency metadata,
- configuration hash,
- analyzer version.

The cache should never contain secrets.

---

# 42. Observability

Production Forge Cloud should provide:

- structured logs,
- metrics,
- tracing,
- error tracking,
- job durations,
- analyzer failures,
- AI latency,
- AI token usage,
- queue depth.

Telemetry must be opt-in where appropriate for OSS/local usage.

---

# 43. Testing Strategy

Forge should use multiple testing layers.

## Unit Tests

- parsers,
- rules,
- scoring,
- configuration,
- graph algorithms.

## Integration Tests

- real repositories,
- package managers,
- Git,
- GitHub API,
- analyzers.

## Fixture Repositories

Maintain curated test projects:

```text
fixtures/
├── next-app/
├── nuxt-app/
├── react-app/
├── fastapi-app/
├── laravel-app/
└── monorepo/
```

## Golden Tests

Expected analyzer output should be snapshot-tested.

## Security Tests

Include malicious repositories and prompt injection fixtures.

---

# 44. CI/CD

Recommended pipeline:

```text
Pull Request
     │
     ├── formatting
     ├── lint
     ├── unit tests
     ├── integration tests
     ├── security scan
     ├── fixture tests
     └── build
             │
             ▼
          Merge
             │
             ▼
       Release Pipeline
             │
       ┌─────┼─────┐
       ▼     ▼     ▼
     CLI   Action  Docs
       │
       ▼
   GitHub Release
```

For developer speed, PR CI should prioritize fast deterministic checks.

Full integration/fixture matrices can run separately.

---

# 45. Release Engineering

Releases should include:

- semantic versioning,
- changelog,
- release notes,
- binaries,
- package artifacts,
- checksums,
- provenance/signatures where feasible.

Supported platforms:

- Linux x64
- Linux ARM64
- macOS ARM64
- macOS x64
- Windows x64

---

# 46. Package Distribution

Potential distribution:

```text
Homebrew
npm wrapper
Cargo
GitHub Releases
Docker
```

Primary recommendation:

**single native binary first**.

Language-specific package managers can act as installation wrappers.

---

# 47. Monorepo Structure

Recommended repository structure:

```text
elcode-forge/
├── apps/
│   ├── dashboard/
│   └── docs/
│
├── crates/
│   ├── forge-core/
│   ├── forge-cli/
│   ├── forge-analyzer/
│   ├── forge-rules/
│   ├── forge-git/
│   ├── forge-security/
│   └── forge-mcp/
│
├── packages/
│   ├── forge-schema/
│   └── forge-ui/
│
├── rules/
│   ├── recommended/
│   └── security/
│
├── fixtures/
├── integrations/
│   └── github-action/
│
├── docs/
├── examples/
├── scripts/
└── .github/
```

A workspace structure can evolve depending on implementation constraints.

---

# 48. Recommended Technology Direction

## Core

**Rust**

Reason:

- high performance,
- native binaries,
- low memory,
- strong CLI ecosystem,
- excellent concurrency,
- appropriate for static/code analysis.

## Dashboard

**SvelteKit + TypeScript + Tailwind CSS + `@elcode/design-system`** (or **Next.js + TypeScript** if separated for ecosystem compatibility), keeping the **Rust Core Engine 100% UI-agnostic**.

## Graph

A dedicated graph visualization library.

## Data

**PostgreSQL** for cloud persistence.

## AI

Provider abstraction with support for:

- OpenAI-compatible APIs,
- Anthropic,
- Gemini,
- OpenRouter,
- Ollama.

## GitHub

GitHub Actions + optional GitHub App.

## MCP

Native MCP server implementation.

---

# 49. Architecture Decision: Rust vs TypeScript

Rust should own the analysis core.

TypeScript should own:

- dashboard,
- documentation site,
- optional API orchestration.

This separation provides:

```text
Rust
= intelligence engine

TypeScript
= product experience
```

It also prevents the dashboard stack from determining the performance characteristics of the core analyzer.

---

# 50. Repository Model

Forge should use a normalized intermediate representation:

```text
Repository
├── ProjectMetadata
├── File
├── Module
├── Symbol
├── Import
├── Export
├── Dependency
├── Test
├── GitChange
├── Finding
├── Rule
└── Relationship
```

This IR becomes the foundation for:

- CLI,
- dashboard,
- MCP,
- AI context,
- reports.

---

# 51. Scoring Engine

Scoring should be deterministic.

Example:

```text
Finding Severity
Critical = -20
High     = -10
Medium   = -5
Low      = -2
Info     = 0
```

However, raw deductions must be normalized per project size and category.

Avoid punishing large repositories simply because they contain more files.

Use:

- density,
- severity,
- affected criticality,
- confidence,
- project baseline.

---

# 52. Risk Engine

Risk is different from health.

Health asks:

> "How healthy is the project?"

Risk asks:

> "How risky is this change?"

Example:

```text
Project Health: 89
PR Risk: HIGH
```

A healthy project can still have a risky PR.

This distinction should remain explicit.

---

# 53. Developer Experience Requirements

Forge must feel useful within one command.

Golden path:

```bash
git clone project
cd project

eforge doctor
```

Within seconds the developer gets meaningful information.

Second command:

```bash
eforge scan
```

Third:

```bash
eforge explain
```

The first-time experience must not require:

- account creation,
- cloud setup,
- API key,
- complex configuration.

---

# 54. Open Source Strategy

## License

A permissive license such as Apache-2.0 or MIT should be evaluated.

If future commercial modules are planned, licensing boundaries must be explicit from the beginning.

Recommended approach:

```text
Forge Core
→ open source

Forge integrations
→ open source where strategically useful

Forge Cloud
→ commercial

Enterprise controls
→ commercial
```

Avoid artificial feature-gating in the core.

---

# 55. Community Strategy

Repository should include:

```text
README.md
CONTRIBUTING.md
CODE_OF_CONDUCT.md
SECURITY.md
GOVERNANCE.md
ARCHITECTURE.md
ROADMAP.md
CHANGELOG.md
LICENSE
```

Issue templates:

- Bug
- Feature
- Analyzer
- Rule
- Security
- Documentation

Discussion categories:

- Ideas
- Help
- Show & Tell
- Architecture

---

# 56. Contributor Experience

A contributor should be able to:

```bash
git clone
cd elcode-forge
eforge dev
```

or the repository's documented equivalent.

Documentation must explain:

- architecture,
- analyzer lifecycle,
- rule development,
- testing,
- release process.

The first contribution should ideally be possible without understanding the entire codebase.

---

# 57. Plugin Architecture

Future analyzers should be modular.

Example:

```text
forge-analyzer-typescript
forge-analyzer-python
forge-analyzer-docker
forge-analyzer-git
forge-analyzer-security
```

The exact plugin mechanism should prioritize:

- security,
- version compatibility,
- deterministic execution,
- performance.

Do not create a dynamic native plugin system until there is a real ecosystem requirement.

A versioned analyzer registry may be safer initially.

---

# 58. CLI Command Matrix

| Command | Purpose |
|---|---|
| `eforge init` | Initialize Forge |
| `eforge doctor` | Quick project diagnosis |
| `eforge scan` | Full deterministic scan |
| `eforge review` | Review current changes |
| `eforge diff` | Change impact |
| `eforge explain` | Explain project/module |
| `eforge ai-review` | AI-assisted change review |
| `eforge security` | Security analysis |
| `eforge dependencies` | Dependency analysis |
| `eforge graph` | Architecture graph |
| `eforge docs` | Documentation generation |
| `eforge onboard` | Developer onboarding |
| `eforge report` | Report generation |
| `eforge rules` | Rule management |
| `eforge mcp` | MCP server |
| `eforge config` | Configuration inspection |

---

# 59. Report Formats

Required:

- terminal,
- Markdown,
- JSON,
- SARIF.

Future:

- HTML,
- PDF,
- GitHub summary,
- machine-readable API.

---

# 60. Production Dashboard Requirements

The production dashboard must support:

- authentication,
- organizations,
- projects,
- repositories,
- role-based access,
- audit logs,
- API keys,
- integrations,
- usage metrics.

Roles:

```text
Owner
Admin
Developer
Viewer
```

---

# 61. Multi-Tenancy

If Forge Cloud is launched:

Tenant isolation is mandatory.

Model:

```text
Organization
 ├── Members
 ├── Projects
 ├── Repositories
 ├── Policies
 ├── Findings
 └── Reports
```

Every resource must have explicit tenant ownership.

No cross-tenant query should be possible by application logic.

Database-level protections should be considered where appropriate.

---

# 62. Enterprise Readiness

Future requirements:

- SSO/SAML,
- SCIM,
- audit logs,
- private deployment,
- self-hosting,
- custom rules,
- organization policies,
- data retention,
- regional data residency,
- private AI providers.

These should not block OSS adoption.

---

# 63. Monetization Strategy

Forge should not monetize the CLI aggressively.

Potential revenue layers:

## Forge Cloud

- hosted dashboard,
- history,
- trends,
- team collaboration.

## Forge Pro

- advanced analytics,
- AI review,
- advanced rules,
- larger projects.

## Forge Enterprise

- SSO,
- audit,
- self-hosting,
- policy management,
- private AI,
- enterprise support.

## Professional Services

ELCODE can offer:

- architecture audits,
- security audits,
- migration,
- developer productivity consulting,
- custom rule packs.

This creates a direct bridge between OSS adoption and ELCODE services.

---

# 64. Growth Flywheel

```text
Open Source
     ↓
Developer Adoption
     ↓
GitHub Stars
     ↓
Community
     ↓
Integrations
     ↓
More Users
     ↓
Forge Cloud
     ↓
Revenue
     ↓
More Development
     ↓
Better Open Source Product
```

---

# 65. Content Strategy

Forge is highly suitable for technical content.

Potential content:

- "How to understand a 500k-line codebase"
- "What AI coding agents don't tell you about technical debt"
- "Building a codebase intelligence engine in Rust"
- "How to detect architecture drift"
- "MCP for codebase intelligence"
- "Why AI-generated code needs a validation layer"
- "Building a production-grade developer CLI"

This supports both:

**ELCODE brand**

and

**Luki personal brand**.

---

# 66. Personal Branding Strategy

The project should visibly connect to Luki without making the project dependent on a personal identity.

GitHub:

```text
ELCODE
└── Forge
```

Personal content:

```text
Luki
└── Founder / Builder
    └── ELCODE Forge
```

This creates:

```text
Luki
  ↓
Technical authority
  ↓
ELCODE
  ↓
Forge
  ↓
Developer community
```

---

# 67. Portfolio Case Study

The final portfolio case study should demonstrate:

### Problem

Modern software is increasingly difficult to understand and validate, especially with AI-assisted development.

### Solution

ELCODE Forge provides repository intelligence, architecture analysis, risk analysis, and AI-ready context.

### Engineering

- Rust analysis engine
- AST
- Git analysis
- static analysis
- rule engine
- GitHub Actions
- MCP
- optional LLM providers
- security architecture
- incremental indexing

### Result

A developer-first open-source intelligence layer.

This is substantially stronger than a conventional CRUD portfolio project.

---

# 68. Product Metrics

## North Star Metric

**Repositories successfully analyzed**

Supporting metrics:

- weekly active repositories,
- scans per repository,
- GitHub Action executions,
- repeat usage,
- findings resolved,
- contributors,
- rule packs,
- MCP usage.

## OSS Metrics

- GitHub stars,
- forks,
- contributors,
- issues,
- discussions,
- package downloads,
- GitHub Action installs.

Avoid optimizing purely for stars.

---

# 69. Quality Metrics

Track:

- false positive rate,
- analyzer crash rate,
- scan duration,
- cache hit rate,
- AI hallucination reports,
- security detection precision,
- rule execution reliability.

A developer tool loses trust quickly if it produces noisy results.

---

# 70. Reliability Targets

Production Cloud target:

- 99.9% API availability,
- 99.9% job completion reliability,
- graceful analyzer failure,
- retryable jobs,
- idempotent processing.

CLI should remain useful even when:

- GitHub is unavailable,
- AI provider is unavailable,
- cloud service is unavailable.

---

# 71. Error Handling

Forge should distinguish:

```text
Analysis failure
Configuration error
User error
Repository error
External service error
AI provider error
Security restriction
```

Errors should be actionable.

Bad:

```text
Error: failed
```

Good:

```text
GitHub API authentication failed.

The configured token does not have pull request read permission.

Run:
eforge github auth
```

---

# 72. Accessibility

Dashboard MUST target WCAG 2.2 AA where applicable.

CLI must support:

- non-color output,
- screen-reader-friendly structure,
- plain output mode.

---

# 73. Internationalization

English is the primary product language.

Architecture should not block future:

- Indonesian,
- Japanese,
- other languages.

ELCODE can initially provide Indonesian documentation selectively.

---

# 74. Documentation Site

Documentation should include:

```text
Getting Started
CLI
Architecture
Analyzers
Rules
GitHub
MCP
AI
Security
Configuration
Contributing
API
Self Hosting
FAQ
```

Each command should have:

- purpose,
- examples,
- configuration,
- output,
- exit codes.

---

# 75. Exit Codes

CLI should use predictable exit codes.

Example:

```text
0 = success
1 = findings above configured threshold
2 = configuration error
3 = execution error
4 = security policy failure
```

This is important for CI.

---

# 76. CI Fast Path

Forge should embrace fast developer feedback.

Recommended modes:

```bash
eforge review --fast
eforge review --changed
eforge scan --incremental
```

Fast mode should:

- analyze changed files,
- resolve direct dependencies,
- run high-value rules,
- avoid unnecessary full repository work.

Full analysis remains available:

```bash
eforge scan --full
```

This directly supports the development philosophy of reducing long feedback loops without sacrificing safety.

---

# 77. Caching Strategy

Cache layers:

```text
File Hash
   ↓
AST Cache
   ↓
Dependency Cache
   ↓
Architecture Cache
   ↓
Rule Results
```

Cache keys must include:

- analyzer version,
- configuration version,
- rule version,
- relevant file hashes.

---

# 78. AI Cost Controls

AI functionality must include:

- token budgets,
- context limits,
- model selection,
- local model support,
- caching,
- structured output,
- configurable temperature where relevant,
- cost reporting.

Example:

```yaml
ai:
  provider: openrouter
  model: ...
  max_tokens: 4000
  budget:
    daily: 1.00
```

---

# 79. AI Quality Controls

AI results should be evaluated using fixtures.

Create benchmark questions:

```text
What is the authentication flow?
What modules depend on UserService?
What is the deployment architecture?
What changed in this PR?
What risks were introduced?
```

Expected answers should be tested.

---

# 80. MCP Security

MCP access must:

- default to read-only,
- expose only necessary tools,
- respect ignored paths,
- respect project boundaries,
- avoid secret files,
- log tool usage in server mode,
- support explicit permissions.

Write-capable MCP tools should be a later, opt-in feature.

---

# 81. Roadmap

## Phase 0 — Foundation

Goal:

Establish architecture and repository quality.

Deliver:

- repository structure,
- architecture document,
- CLI shell,
- config schema,
- IR model,
- CI,
- test infrastructure,
- release process.

---

## Phase 1 — Codebase Intelligence

Goal:

Make Forge useful without AI.

Deliver:

- project discovery,
- language detection,
- dependency analysis,
- Git analysis,
- architecture graph,
- findings,
- health score,
- JSON/Markdown output.

---

## Phase 2 — Developer Workflow

Deliver:

- `eforge doctor`,
- `eforge review`,
- `eforge diff`,
- incremental analysis,
- fast mode,
- SARIF,
- GitHub Action.

---

## Phase 3 — Documentation & Onboarding

Deliver:

- `eforge onboard`,
- `eforge docs`,
- architecture documentation,
- module documentation,
- risk reports.

---

## Phase 4 — AI Intelligence

Deliver:

- provider abstraction,
- `eforge explain`,
- AI review,
- context retrieval,
- local LLM support,
- cost controls.

---

## Phase 5 — MCP

Deliver:

- MCP server,
- repository intelligence tools,
- AI agent integration,
- security boundaries.

---

## Phase 6 — Dashboard

Deliver:

- project dashboard,
- architecture visualization,
- findings,
- trends,
- reports.

---

## Phase 7 — Ecosystem

Deliver:

- custom rules,
- rule packs,
- analyzer packages,
- community contributions,
- registry.

---

## Phase 8 — Forge Cloud

Deliver:

- accounts,
- organizations,
- repository synchronization,
- historical analysis,
- collaboration,
- team dashboards.

---

## Phase 9 — Enterprise

Deliver:

- SSO,
- SCIM,
- audit logs,
- self-hosting,
- private AI,
- policies,
- support.

---

# 82. Version Strategy

Suggested:

```text
0.1  Foundation
0.2  Codebase Scanner
0.3  Architecture
0.4  Rules + Health
0.5  GitHub Action
0.6  Change Intelligence
0.7  AI
0.8  MCP
0.9  Dashboard
1.0  Production Core
```

After 1.0:

```text
1.x ecosystem
2.x cloud/platform
```

Do not wait for every planned feature before releasing 0.x versions.

---

# 83. Definition of Done — Production Core

Forge Core is production-ready when:

- CLI works on supported platforms,
- scans are deterministic,
- analyzer crashes are isolated,
- cache is reliable,
- findings have confidence/severity,
- output supports JSON/SARIF/Markdown,
- CI integration is stable,
- security tests pass,
- malicious repository fixtures pass safely,
- documentation is complete,
- release artifacts are reproducible,
- versioning is stable,
- telemetry/privacy behavior is documented.

---

# 84. Definition of Done — Production Platform

Forge Platform is production-ready when:

- authentication is secure,
- tenant isolation is tested,
- audit logging works,
- jobs are idempotent,
- backups are configured,
- observability is operational,
- incident response exists,
- data retention is documented,
- security review is completed,
- disaster recovery is tested.

---

# 85. Non-Goals

Forge will NOT initially attempt to become:

- a full IDE,
- a replacement for GitHub,
- a replacement for Jira,
- a general project-management platform,
- a general AI coding agent,
- a universal vulnerability scanner,
- a full observability platform,
- a deployment platform.

These boundaries are important.

---

# 86. Competitive Strategy

Forge should integrate with existing tools rather than attempting to replace them.

Examples:

```text
GitHub
GitLab
VS Code
Cursor
Codex
Claude Code
OpenCode
Ollama
Semgrep
Trivy
OSV
```

The strategic role is:

> **intelligence and context layer**

rather than:

> **replacement for every developer tool**.

---

# 87. Biggest Product Risks

## Risk 1 — Scope Explosion

Mitigation:

- core engine first,
- strict command boundaries,
- no cloud before CLI value.

## Risk 2 — Too Many False Positives

Mitigation:

- confidence scoring,
- evidence,
- conservative rules,
- benchmark repositories.

## Risk 3 — AI Hype Without Real Value

Mitigation:

- deterministic core,
- AI optional,
- AI only after context extraction.

## Risk 4 — Competing with Too Many Tools

Mitigation:

- focus on repository intelligence,
- integration instead of replacement.

## Risk 5 — Performance

Mitigation:

- Rust,
- incremental indexing,
- cache,
- parallel analysis.

## Risk 6 — Security

Mitigation:

- treat repository as untrusted,
- sandbox external tools,
- least privilege,
- secure defaults.

## Risk 7 — Solo Founder Burnout

Mitigation:

- open-source modularity,
- small release increments,
- community analyzers,
- avoid building cloud too early.

---

# 88. Recommended First Implementation

The first serious implementation should be:

```text
eforge doctor
eforge scan
eforge review
```

with:

```text
✓ project detection
✓ dependency detection
✓ architecture inference
✓ Git diff analysis
✓ health score
✓ findings
✓ JSON output
✓ Markdown output
```

Do NOT start with:

- dashboard,
- authentication,
- billing,
- multi-tenancy,
- elaborate AI.

The first milestone is:

> **A developer can clone any supported project, run `eforge doctor`, and immediately learn something useful.**

---

# 89. Suggested Initial Repository

Recommended:

```text
github.com/elcode-id/forge
```

Alternative:

```text
github.com/elcode-id/elcode-forge
```

For discoverability, **`elcode-forge`** is preferable as the repository name.

Primary binary:

```bash
eforge
```

Secondary alias (package distribution):

```bash
elcode-forge
```

Package/organization:

```text
ELCODE Forge
```

---

# 90. Brand System

Primary:

**ELCODE Forge**

Short:

**Forge**

CLI:

```text
eforge
```

*(Alias: `elcode-forge`)*

Tagline:

> **Know your code. Ship with confidence.**

Alternative:

> **Codebase intelligence for modern development.**

Brand personality:

- technical,
- precise,
- confident,
- minimal,
- open,
- engineering-first.

Avoid:

- excessive AI branding,
- cyberpunk clichés,
- generic "AI magic" language.

---

# 91. Launch Strategy

## Stage 1

Private development.

Build against:

- personal projects,
- ELCODE Studio OS,
- SKALA-like applications,
- Nuxt projects,
- Next.js projects,
- FastAPI projects.

## Stage 2

Public GitHub repository.

Release:

```text
v0.1.0
```

with excellent README and demo.

## Stage 3

Technical launch content.

Publish:

1. Problem statement
2. Architecture article
3. Demo
4. Benchmark
5. Roadmap

## Stage 4

Community validation.

Target:

- developers,
- open-source maintainers,
- AI-agent users,
- software agencies,
- technical consultants.

---

# 92. Dogfooding Strategy

ELCODE Forge MUST be used to analyze itself.

The Forge repository should have CI:

```text
Forge
  ↓
Forge analyzes Forge
  ↓
Findings
  ↓
PR review
  ↓
Merge
```

This creates a strong engineering story:

> "Forge is built and continuously validated by Forge."

This should become a major project principle.

---

# 93. Dogfood Targets

Forge should analyze:

1. itself,
2. ELCODE Studio OS,
3. ELCODE websites,
4. sample Nuxt project,
5. sample Next project,
6. sample FastAPI project,
7. deliberately vulnerable fixture.

This produces realistic benchmark data.

---

# 94. Long-Term Vision

The ultimate Forge architecture:

```text
                         ELCODE FORGE
                              │
              ┌───────────────┼────────────────┐
              │               │                │
          Understand       Validate          Assist
              │               │                │
          Architecture     Security          AI
          Dependencies     Quality            MCP
          Ownership        Changes            Agents
              │               │                │
              └───────────────┼────────────────┘
                              │
                       Engineering Graph
                              │
                 ┌────────────┼────────────┐
                 │            │            │
              Developer      CI/CD       AI Agent
                 │            │            │
                 └────────────┼────────────┘
                              │
                       Ship with Confidence
```

The deepest product asset is not the dashboard.

It is the **engineering graph and repository intelligence model**.

That model can power:

- CLI,
- dashboard,
- AI,
- MCP,
- GitHub,
- reports,
- future automation.

---

# 95. Final Strategic Recommendation

ELCODE Forge should be treated as a flagship engineering project, not a side utility.

The objective is not:

> "Get many GitHub stars."

The objective is:

> **Build a genuinely useful developer tool that demonstrates the engineering depth of ELCODE and Luki.**

The project should communicate:

```text
ELCODE
  = builds software products

Luki
  = understands modern software engineering

Forge
  = proves it publicly
```

The strongest positioning is:

> **ELCODE Forge — Know your code. Ship with confidence.**

And the strategic sequence should be:

```text
Open Source Core
       ↓
Developer Adoption
       ↓
Community
       ↓
AI / MCP Ecosystem
       ↓
Forge Cloud
       ↓
Enterprise
       ↓
ELCODE Developer Platform
```

---

# Appendix A — Initial Backlog

## Epic: Foundation

- [ ] Repository bootstrap
- [ ] Rust workspace
- [ ] CLI framework
- [ ] Config schema
- [ ] Logging
- [ ] Error model
- [ ] CI
- [ ] Release workflow

## Epic: Scanner

- [ ] File scanner
- [ ] Ignore handling
- [ ] Language detection
- [ ] Framework detection
- [ ] Package manager detection
- [ ] Git detection

## Epic: Dependency Intelligence

- [ ] package manifest parser
- [ ] dependency graph
- [ ] unused dependency detection
- [ ] duplicate version detection
- [ ] vulnerability provider interface

## Epic: Architecture

- [ ] import graph
- [ ] module graph
- [ ] circular dependency detection
- [ ] architecture inference
- [ ] graph export

## Epic: Findings

- [ ] finding schema
- [ ] severity
- [ ] confidence
- [ ] evidence
- [ ] recommendations

## Epic: Scoring

- [ ] health model
- [ ] risk model
- [ ] configurable weights
- [ ] explainability

## Epic: Git

- [ ] diff parser
- [ ] change analysis
- [ ] hotspot detection
- [ ] impact analysis

## Epic: Security

- [ ] secret detection
- [ ] security rules
- [ ] external scanner interface
- [ ] SARIF

## Epic: GitHub

- [ ] Action
- [ ] PR checks
- [ ] annotations
- [ ] summary comments

## Epic: AI

- [ ] provider abstraction
- [ ] context builder
- [ ] explain
- [ ] AI review
- [ ] token budget
- [ ] local model support

## Epic: MCP

- [ ] MCP server
- [ ] repository tools
- [ ] permissions
- [ ] security boundary

## Epic: Dashboard

- [ ] project overview
- [ ] health
- [ ] architecture
- [ ] findings
- [ ] dependencies
- [ ] changes
- [ ] reports

## Epic: Ecosystem

- [ ] rule SDK
- [ ] rule registry
- [ ] analyzer registry
- [ ] contributor docs

---

# Appendix B — Initial Success Criteria

The first public release should satisfy:

```text
A developer can:

1. Install Forge in under 1 minute.
2. Run `eforge doctor`.
3. Run `eforge scan`.
4. Understand the project architecture.
5. See meaningful findings.
6. Run `eforge review` against a Git diff.
7. Use the result in CI.
8. Export JSON/SARIF/Markdown.
9. Run everything locally without an account.
10. Understand exactly why Forge produced each result.
```

If these ten conditions are satisfied, Forge has a strong foundation for becoming a serious open-source developer tool.

---

# Appendix C — Guiding Principle

> **Don't build an AI that writes more code.**
>
> **Build the intelligence layer that helps humans and AI understand the code that already exists.**

That is the strategic heart of ELCODE Forge.
