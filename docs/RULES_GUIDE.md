# Declarative Custom Rules & Rule Packs Guide

**ELCODE Forge (`eforge`)** provides a declarative, deterministic rules engine designed to enforce architectural boundaries, code quality standards, and security disciplines without arbitrary script execution or runtime compromises.

Operating under the **ELCODE Studio Manifesto** (`https://elcode.id/`):
- *"Software should feel like an instrument, not a compromise."*
- *"Deterministic before generative."*
- *"We prefer precision over decoration."*

Every rule evaluated by ELCODE Forge is deterministic, explainable, and zero-panic guarded.

---

## 1. Architectural Philosophy & Threat Model

Unlike linters that execute arbitrary JavaScript or Python plugins in the developer's process, ELCODE Forge evaluates declarative YAML or JSON specifications using pure, high-performance Rust matchers:

```
┌─────────────────────────────────────────────────────────────────┐
│ Target Repository (Untrusted Hostile Input)                     │
└───────────────────────────────┬─────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│ Path-Guarded Rule Loader (crates/forge-rules)                   │
│ 1. Canonical Path Resolution (path_guard::resolve_safe_path)    │
│ 2. Schema Validation (Category, Severity, Confidence, Matchers) │
│ 3. Inheritance Hierarchy Resolution (extends + custom_packs)    │
└───────────────────────────────┬─────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│ Deterministic Matcher Pipeline                                  │
│ ├── forbidden_imports        (DAG & Layer Boundary Enforcement) │
│ ├── required_companion       (Companion Test Coverage)          │
│ ├── file_naming              (Convention & Suffix Patterns)     │
│ ├── file_size                (Line & Byte Thresholds)           │
│ ├── regex_pattern            (Prohibited Dynamic Tokens)        │
│ └── dependency_restriction   (Unpinned & Deprecated Packages)   │
└───────────────────────────────┬─────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│ Unified Findings & Intelligence Delivery                        │
│ ├── Terminal Table / JSON / SARIF (`eforge scan`, `rules list`) │
│ ├── Interactive Dashboard (`/rules`, `/findings`)               │
│ └── MCP Agent Envelope (`eforge_get_rules`)                     │
└─────────────────────────────────────────────────────────────────┘
```

### Safety Invariants:
1. **Zero-Panic Policy:** Invalid YAML, malformed patterns, or non-existent target files produce structured diagnostics with line numbers rather than crashes.
2. **Path Traversal Protection:** All rule files and scanned sources are constrained to the canonical repository root via `resolve_safe_path`. Specifying `../` outside the repository is strictly rejected.
3. **No Code Execution:** Rules are declarative data models. Analyzing a repository never runs foreign package scripts or arbitrary shell hooks.

---

## 2. Declarative Rule Schema Specification

Custom rules are defined in YAML or JSON files located in `.eforge/rules/` or referenced in `eforge.config.yaml`.

### Complete Specification

```yaml
version: 1

rules:
  - id: "arch/no-db-in-ui"              # Required. Format: <category>/<slug>
    name: "No Direct DB in UI"          # Required. Human-readable rule title
    category: "architecture"            # Required: architecture | quality | security | dependencies
    severity: "critical"                # Required: critical | high | medium | low | info
    confidence: "certain"               # Required: certain | high | medium | low
    enabled: true                       # Optional. Default: true
    description: "UI components must not import database drivers directly."
    remediation: "Route data queries through service layer in src/services/."
    match:                              # Required: Exactly one matcher definition
      type: "forbidden_imports"
      files: ["src/ui/**", "src/components/**"]
      forbidden: ["src/db/**", "pg", "mysql2", "@prisma/client"]
```

### Rule Categories
- `architecture`: Layer separation, circular dependency prevention, and dependency flow constraints.
- `quality`: Companion test verification, file length limits, and file naming conventions.
- `security`: Detection of dangerous patterns (e.g., `eval()`, `child_process.exec()`, debugger statements).
- `dependencies`: Prohibited packages, unpinned wildcard versions (`*`, `latest`).

---

## 3. The Six Deterministic Matchers

### 3.1. `forbidden_imports` — Architectural Boundary Enforcement
Enforces architectural boundaries by prohibiting specific modules or packages from being imported by designated source files.

```yaml
  - id: "arch/isolate-domain-layer"
    name: "Pure Domain Model"
    category: "architecture"
    severity: "critical"
    confidence: "certain"
    description: "Domain entities must have zero dependencies on infrastructure or UI."
    remediation: "Invert dependencies using repository interfaces."
    match:
      type: "forbidden_imports"
      files: ["src/domain/**"]
      forbidden: ["src/infra/**", "src/ui/**", "axios", "express"]
```

### 3.2. `required_companion` — Companion Test Verification
Ensures that critical components (such as application services or API routes) have corresponding test companion files.

```yaml
  - id: "quality/service-test-companion"
    name: "Service Test Requirement"
    category: "quality"
    severity: "high"
    confidence: "certain"
    description: "All services must have a corresponding test companion file."
    remediation: "Create companion test in tests/{name}.test.ts."
    match:
      type: "required_companion"
      files: ["src/services/**"]
      companion: "tests/{name}.test.ts"
```
*Token `{name}` automatically interpolates the source file base name (e.g. `UserService` -> `tests/UserService.test.ts`).*

### 3.3. `file_naming` — Naming Convention Discipline
Validates file naming patterns across designated directories using suffixes, prefixes, or case conventions.

```yaml
  - id: "quality/controller-suffix"
    name: "Controller Suffix Convention"
    category: "quality"
    severity: "medium"
    confidence: "certain"
    description: "Controllers must end with Controller.ts."
    remediation: "Rename file to follow *Controller.ts pattern."
    match:
      type: "file_naming"
      files: ["src/controllers/**"]
      convention: "*Controller.ts"
```

### 3.4. `file_size` — Line Count & File Size Thresholds
Detects overly complex "god modules" before they become maintenance hotspots.

```yaml
  - id: "quality/max-module-lines"
    name: "Module Line Limit"
    category: "quality"
    severity: "low"
    confidence: "certain"
    description: "Source files should not exceed 300 lines of code."
    remediation: "Decompose module into smaller, cohesive units."
    match:
      type: "file_size"
      files: ["src/**"]
      max_lines: 300
```

### 3.5. `regex_pattern` — Prohibited Syntax & Antipatterns
Scans source files for dangerous syntax or deprecated API calls using deterministic regex patterns.

```yaml
  - id: "security/no-dynamic-eval"
    name: "Disallow Dynamic Code Evaluation"
    category: "security"
    severity: "critical"
    confidence: "certain"
    description: "Prevent dynamic eval() calls to avoid remote code execution risks."
    remediation: "Use structured JSON parsers or static dispatch tables."
    match:
      type: "regex_pattern"
      files: ["**/*.ts", "**/*.js"]
      pattern: 'eval\('
```

### 3.6. `dependency_restriction` — Supply-Chain & Manifest Governance
Enforces package management discipline by disallowing deprecated libraries and unpinned wildcard version specifiers.

```yaml
  - id: "deps/prohibit-deprecated-packages"
    name: "Prohibit Deprecated Packages"
    category: "dependencies"
    severity: "high"
    confidence: "certain"
    description: "Prohibit compromised or deprecated dependencies."
    remediation: "Replace deprecated package with modern maintained alternative."
    match:
      type: "dependency_restriction"
      disallowed_packages: ["left-pad", "event-stream", "moment", "wildcard-version"]
```

---

## 4. Rule Packs & Inheritance Hierarchy

ELCODE Forge provides four built-in rule packs that can be extended in `eforge.config.yaml`:

| Pack Name | Focus | Key Rules Included |
| :--- | :--- | :--- |
| **`recommended`** | Standard Baseline | `arch/no-direct-db-in-ui`, `arch/no-circular-dependencies`, `quality/require-service-tests`, `quality/max-file-lines: 300` |
| **`strict`** | Elevated Discipline | `recommended` rules with 250-line file limit, strict companion tests, and wildcard version prohibition |
| **`security`** | Code Hardening | Disallows `eval()`, `child_process.exec()`, `debugger`, `dangerouslySetInnerHTML`, and known deprecated packages |
| **`clean-architecture`** | Hexagonal / Onion | Enforces strict 4-tier layer boundaries (`ui-no-infra`, `domain-pure`, `infra-no-ui`, `ui-no-db`) |

### Configuring Pack Inheritance

In `eforge.config.yaml`:

```yaml
version: 1

rules:
  # 1. Extend one or more packs
  extends:
    - recommended
    - clean-architecture
    - security

  # 2. Add local custom rule files or directories
  custom_packs:
    - .eforge/rules/team-rules.yaml
    - rules/

  # 3. Apply per-rule overrides
  quality/max-file-lines:
    max_lines: 400
    severity: medium
```

### Inheritance Precedence:
1. **Built-in Defaults** (Lowest precedence)
2. **Extended Packs** (Evaluated in order of appearance)
3. **Custom Local Rules** (`.eforge/rules/*`)
4. **Per-Rule Config Overrides** (Highest precedence)

---

## 5. CLI Rules Management Workflow

### 5.1. Initialize Starter Rules Template
Generate a starter template in `.eforge/rules/custom-rules.yaml` containing comprehensive examples for all 6 matchers:

```bash
eforge rules init
```

### 5.2. Inspect Active Rule Catalog
View all active rules, their categories, severity, origin pack, and enabled status:

```bash
# Terminal table view
eforge rules list

# Filter by category
eforge rules list --category architecture

# Filter only enabled rules
eforge rules list --enabled-only

# JSON format (ideal for automation & CI)
eforge rules list --format json
```

### 5.3. Check Rule Syntax Without Scanning
Verify rule files against the schema to catch typos or syntax errors early:

```bash
# Verify all rules in .eforge/rules/
eforge rules check

# Verify a specific file
eforge rules check .eforge/rules/custom-rules.yaml
```

### 5.4. Run Verification in CI Gate
Fail CI builds when architectural or custom rule violations occur:

```bash
eforge scan --fail-on high
```

---

## 6. Model Context Protocol (MCP) Integration

AI agents operating over Model Context Protocol (MCP) can inspect active rules via the **`eforge_get_rules`** tool:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "eforge_get_rules",
    "arguments": {
      "category": "architecture",
      "enabled_only": true
    }
  }
}
```

The response is safely enveloped inside `UntrustedEnvelope` with automatic prompt-injection defense signal detection, ensuring AI coding agents can discover and respect repository architectural rules reliably.

---

## 7. Interactive Dashboard

The ELCODE Forge web dashboard features a dedicated **Rules Catalog** screen (`/rules`):
- Filter rules by category and origin pack.
- Live violation counters per rule in the current project.
- Clickable badges linking directly to matching violations in `/findings`.
- Remediation guidance display for every rule.
