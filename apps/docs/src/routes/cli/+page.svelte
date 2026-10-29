<svelte:head>
  <title>CLI Reference & Commands Catalog | ELCODE Forge Documentation</title>
</svelte:head>

<script lang="ts">
  type CommandInfo = {
    name: string;
    category: "Diagnostics" | "Architecture" | "Security" | "AI" | "Governance" | "Compliance" | "Integration";
    summary: string;
    usage: string;
    flags: { flag: string; desc: string; default?: string }[];
    examples: string[];
    exitCodes: { code: string; desc: string }[];
  };

  const commands: CommandInfo[] = [
    {
      name: "eforge init",
      category: "Diagnostics",
      summary: "Initialize eforge.config.yaml and .eforge/ workspace metadata and BLAKE3 incremental cache directory.",
      usage: "eforge init [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--force", desc: "Overwrite existing eforge.config.yaml if present", default: "false" },
      ],
      examples: [
        "eforge init",
        "eforge init --path ./services/api",
        "eforge init --force",
      ],
      exitCodes: [
        { code: "0", desc: "Initialized eforge.config.yaml and .eforge/ directory" },
        { code: "2", desc: "Configuration already exists (when --force is not passed)" },
      ],
    },
    {
      name: "eforge doctor",
      category: "Diagnostics",
      summary: "Run a fast (< 5s) repository diagnostic check evaluating file inventory, detected languages/frameworks, manifest & lockfile hygiene (Node, Python, Go, Rust, PHP), test suite presence, rules, and secret hygiene.",
      usage: "eforge doctor [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown, sarif, github", default: "terminal" },
      ],
      examples: [
        "eforge doctor",
        "eforge doctor --path . --format json",
        "eforge doctor --format markdown",
      ],
      exitCodes: [
        { code: "0", desc: "All diagnostic checks completed with zero fatal failures" },
        { code: "1", desc: "One or more repository checks failed or breached thresholds" },
      ],
    },
    {
      name: "eforge scan",
      category: "Diagnostics",
      summary: "Execute a full or BLAKE3 incremental polyglot architecture, dependency, declarative rule, OSV vulnerability, secret, and 8-dimension health scan.",
      usage: "eforge scan [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown, sarif, github", default: "terminal" },
        { flag: "--full", desc: "Force a full repository scan bypassing cache", default: "false" },
        { flag: "--incremental", desc: "Use .eforge/metadata.json BLAKE3 content hashes to inspect changed files", default: "false" },
        { flag: "--fail-on <SEVERITY>", desc: "Minimum finding severity to return non-zero exit code (critical|high|medium|low|info)" },
        { flag: "-o, --output <PATH>", desc: "Optional file path to write the rendered report" },
      ],
      examples: [
        "eforge scan",
        "eforge scan --incremental --fail-on high",
        "eforge scan --format sarif -o eforge.sarif",
        "eforge scan --format github --fail-on high",
      ],
      exitCodes: [
        { code: "0", desc: "Scan succeeded with no findings at or above --fail-on threshold" },
        { code: "1", desc: "Findings met or exceeded the configured --fail-on severity threshold" },
        { code: "2", desc: "Invalid configuration or severity threshold syntax" },
      ],
    },
    {
      name: "eforge review",
      category: "Diagnostics",
      summary: "Analyze working-tree Git diff blast radius, direct & transitive module impact, untested file warnings, change risk level (Low|Medium|High|Critical), and findings on modified files.",
      usage: "eforge review [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown, sarif, github", default: "terminal" },
        { flag: "--fast", desc: "Run fast diff-scoped mode", default: "false" },
        { flag: "--changed", desc: "Scope analysis strictly to changed files in working tree", default: "false" },
        { flag: "--fail-on <SEVERITY>", desc: "Minimum finding severity to return non-zero exit code (critical|high|medium|low|info)" },
      ],
      examples: [
        "eforge review --fast",
        "eforge review --fast --changed --fail-on high",
        "eforge review --format markdown",
      ],
      exitCodes: [
        { code: "0", desc: "Diff review passed below threshold" },
        { code: "1", desc: "Diff findings met or exceeded --fail-on severity" },
      ],
    },
    {
      name: "eforge diff",
      category: "Diagnostics",
      summary: "Inspect working-tree unified diff (git diff HEAD) and affected architecture modules, or compare current repository state against a remote baseline snapshot (--remote).",
      usage: "eforge diff [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--remote", desc: "Compare findings and health score against cached remote baseline (.eforge/baseline.json)", default: "false" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown, sarif, github", default: "terminal" },
      ],
      examples: [
        "eforge diff",
        "eforge diff --format markdown",
        "eforge diff --remote --format json",
      ],
      exitCodes: [
        { code: "0", desc: "Diff impact or remote baseline delta computed" },
      ],
    },
    {
      name: "eforge graph",
      category: "Architecture",
      summary: "Export the inferred polyglot module architecture graph, inter-module import edges, high-centrality modules, and Tarjan SCC circular dependency cycles.",
      usage: "eforge graph [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--format <FMT>", desc: "terminal (module list), json (ArchitectureGraph), markdown (Mermaid flowchart TD)", default: "terminal" },
      ],
      examples: [
        "eforge graph",
        "eforge graph --format markdown",
        "eforge graph --format json",
      ],
      exitCodes: [
        { code: "0", desc: "Architecture graph resolved and rendered" },
      ],
    },
    {
      name: "eforge dependencies",
      category: "Architecture",
      summary: "Inspect package manifest dependencies, lockfile versions, open-source licenses, unused packages, duplicate declarations, and unpinned versions across npm, PyPI, Go, Cargo, and Composer.",
      usage: "eforge dependencies [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown, sarif, github", default: "terminal" },
      ],
      examples: [
        "eforge dependencies",
        "eforge dependencies --format json",
        "eforge dependencies --format markdown",
      ],
      exitCodes: [
        { code: "0", desc: "Dependency report generated" },
      ],
    },
    {
      name: "eforge onboard",
      category: "Architecture",
      summary: "Generate the deterministic 5-file living developer onboarding and architecture documentation bundle (architecture.md, modules.md, dependencies.md, onboarding.md, risks.md).",
      usage: "eforge onboard [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "-o, --output <DIR>", desc: "Output directory for generated Markdown docs", default: "<path>/docs/forge" },
      ],
      examples: [
        "eforge onboard",
        "eforge onboard --path . -o ./docs/architecture",
      ],
      exitCodes: [
        { code: "0", desc: "5 Markdown documentation files written deterministically" },
      ],
    },
    {
      name: "eforge docs",
      category: "Architecture",
      summary: "Alias for eforge onboard — regenerate deterministic architecture, module, dependency, onboarding, and risk documentation.",
      usage: "eforge docs [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "-o, --output <DIR>", desc: "Output directory for generated Markdown docs", default: "<path>/docs/forge" },
      ],
      examples: [
        "eforge docs",
        "eforge docs -o ./docs/forge",
      ],
      exitCodes: [
        { code: "0", desc: "Documentation bundle regenerated" },
      ],
    },
    {
      name: "eforge security",
      category: "Security",
      summary: "Run focused secret detection (verified credentials + Shannon entropy), unsafe JS/TS/Python/PHP code pattern analysis, and polyglot OSV dependency vulnerability auditing.",
      usage: "eforge security [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown, sarif, github", default: "terminal" },
        { flag: "--audit", desc: "Run deep dependency vulnerability audit against OSV/GHSA advisories", default: "false" },
        { flag: "--offline", desc: "Run in offline mode using embedded and cached (.eforge/cache/osv/) advisories only", default: "false" },
        { flag: "--fail-on <SEVERITY>", desc: "Minimum finding severity to trigger security gate failure (critical|high|medium|low|info)" },
      ],
      examples: [
        "eforge security",
        "eforge security --audit --fail-on high",
        "eforge security --audit --offline --format sarif",
      ],
      exitCodes: [
        { code: "0", desc: "No security findings at or above threshold" },
        { code: "4", desc: "SecurityPolicyFailure: findings breached --fail-on severity threshold" },
      ],
    },
    {
      name: "eforge sbom",
      category: "Security",
      summary: "Export a deterministic Software Bill of Materials (SBOM) in OASIS CycloneDX v1.5 JSON or ISO SPDX v2.3 JSON format with canonical Package URLs (PURL) and BLAKE3 serial numbers.",
      usage: "eforge sbom [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--format <FMT>", desc: "SBOM specification format: cyclonedx, spdx", default: "cyclonedx" },
        { flag: "-o, --output <PATH>", desc: "Optional file path to write the rendered SBOM JSON" },
      ],
      examples: [
        "eforge sbom --format cyclonedx -o sbom.cdx.json",
        "eforge sbom --format spdx -o sbom.spdx.json",
      ],
      exitCodes: [
        { code: "0", desc: "SBOM generated deterministically" },
      ],
    },
    {
      name: "eforge explain",
      category: "AI",
      summary: "Explain the structural role, files, outgoing imports, incoming dependents, exported symbols, and findings for the project or a specific module/file path prefix, with optional grounded AI synthesis.",
      usage: "eforge explain [TARGET] [OPTIONS]",
      flags: [
        { flag: "[TARGET]", desc: "Optional module ID or file path prefix (defaults to '.' for full project)", default: "." },
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown, sarif, github", default: "terminal" },
        { flag: "--ai", desc: "Augment deterministic structural facts with grounded AI synthesis", default: "false" },
        { flag: "--provider <PROVIDER>", desc: "AI provider override: openai, anthropic, gemini, mock" },
        { flag: "--model <MODEL>", desc: "Model identifier override (e.g. gpt-4o, qwen2.5-coder:7b)" },
        { flag: "--dry-run", desc: "Output assembled ContextPack and token estimates without calling AI provider", default: "false" },
        { flag: "--no-cache", desc: "Bypass .eforge/ai-cache/ response cache", default: "false" },
      ],
      examples: [
        "eforge explain",
        "eforge explain src/services --format markdown",
        "eforge explain src/services --ai",
        "eforge explain src/services --ai --dry-run",
      ],
      exitCodes: [
        { code: "0", desc: "Structural explanation and verified AI claims rendered" },
      ],
    },
    {
      name: "eforge ai-review",
      category: "AI",
      summary: "Analyze changes with deterministic diff/PR review augmented by grounded AI review items (Confirmed, Probable, Suggestion) verified via apply_deterministic_guard.",
      usage: "eforge ai-review [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--base <REF>", desc: "Base Git reference for diff comparison (e.g. origin/main or HEAD~1)" },
        { flag: "--head <REF>", desc: "Head Git reference for diff comparison (defaults to working tree)" },
        { flag: "--fast", desc: "Run fast diff-scoped mode", default: "false" },
        { flag: "--fail-on <SEVERITY>", desc: "Minimum deterministic finding severity to fail on (critical|high|medium|low|info)" },
        { flag: "--provider <PROVIDER>", desc: "AI provider override: openai, anthropic, gemini, mock" },
        { flag: "--model <MODEL>", desc: "Model identifier override" },
        { flag: "--dry-run", desc: "Output ContextPack and token estimates without network calls", default: "false" },
        { flag: "--no-cache", desc: "Bypass .eforge/ai-cache/ response cache", default: "false" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown, sarif, github", default: "terminal" },
      ],
      examples: [
        "eforge ai-review --dry-run",
        "eforge ai-review --base origin/develop --fast --fail-on high",
        "eforge ai-review --provider openai --model qwen2.5-coder:7b",
      ],
      exitCodes: [
        { code: "0", desc: "Review completed; deterministic findings below threshold" },
        { code: "1", desc: "Deterministic findings breached --fail-on (AI never alters exit codes)" },
      ],
    },
    {
      name: "eforge ai status",
      category: "AI",
      summary: "Inspect effective AI configuration, masked credential status, daily budget, and optionally test live provider connectivity and latency (--ping).",
      usage: "eforge ai status [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--ping", desc: "Test live provider reachability and measure round-trip latency", default: "false" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json", default: "terminal" },
      ],
      examples: [
        "eforge ai status",
        "eforge ai status --ping",
        "eforge ai status --format json",
      ],
      exitCodes: [
        { code: "0", desc: "AI status inspected (zero network calls unless --ping is set)" },
      ],
    },
    {
      name: "eforge ai usage",
      category: "AI",
      summary: "Summarize AI token consumption, response cache hits, accumulated USD spend, and remaining daily budget from .eforge/ai-usage.jsonl.",
      usage: "eforge ai usage [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--format <FMT>", desc: "Output format: terminal, json", default: "terminal" },
      ],
      examples: [
        "eforge ai usage",
        "eforge ai usage --format json",
      ],
      exitCodes: [
        { code: "0", desc: "Usage ledger summarized safely" },
      ],
    },
    {
      name: "eforge rules list",
      category: "Governance",
      summary: "List all registered built-in, pack-extended, and custom declarative rules with their ID, category, severity, origin pack, and enabled status (default subcommand of eforge rules).",
      usage: "eforge rules [list] [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "-c, --category <CAT>", desc: "Filter by category: architecture, quality, security, dependencies" },
        { flag: "--enabled-only", desc: "Show only enabled rules", default: "false" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge rules",
        "eforge rules list --category architecture --enabled-only",
        "eforge rules list --format json",
      ],
      exitCodes: [
        { code: "0", desc: "Rule catalog rendered" },
      ],
    },
    {
      name: "eforge rules check",
      category: "Governance",
      summary: "Validate syntax and matcher invariants of declarative YAML/JSON rule files without executing a full repository scan.",
      usage: "eforge rules check [PATH] [OPTIONS]",
      flags: [
        { flag: "[PATH]", desc: "Optional path to a specific rule file or directory (defaults to configured paths and .eforge/rules)" },
        { flag: "-p, --repo <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge rules check",
        "eforge rules check .eforge/rules/custom-rules.yaml",
      ],
      exitCodes: [
        { code: "0", desc: "All declarative rule files are syntactically valid" },
        { code: "2", desc: "ConfigurationError: invalid rule ID, unknown matcher, or missing required field" },
      ],
    },
    {
      name: "eforge rules init",
      category: "Governance",
      summary: "Generate a starter declarative rule template in .eforge/rules/custom-rules.yaml with examples for all 6 deterministic matchers.",
      usage: "eforge rules init [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "-o, --output <PATH>", desc: "Output file path", default: "<path>/.eforge/rules/custom-rules.yaml" },
        { flag: "--force", desc: "Force overwrite if destination file already exists", default: "false" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge rules init",
        "eforge rules init --force",
      ],
      exitCodes: [
        { code: "0", desc: "Starter custom-rules.yaml template created" },
      ],
    },
    {
      name: "eforge hook install",
      category: "Governance",
      summary: "Install native sub-200ms Git hook scripts into .git/hooks/ while preserving any pre-existing foreign hooks as <hook>.forge-backup.",
      usage: "eforge hook install [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--hook <TYPE>", desc: "Hook type to install: pre-commit, pre-push, all", default: "all" },
        { flag: "--force", desc: "Force overwrite of existing hook files", default: "false" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge hook install",
        "eforge hook install --hook pre-commit",
      ],
      exitCodes: [
        { code: "0", desc: "Git hooks installed with 0o755 executable permissions" },
      ],
    },
    {
      name: "eforge hook run",
      category: "Governance",
      summary: "Execute a Git hook lifecycle check on staged files (pre-commit) or unpushed commits (pre-push). Bypassable via FORGE_SKIP_HOOK=1.",
      usage: "eforge hook run <HOOK> [OPTIONS]",
      flags: [
        { flag: "<HOOK>", desc: "Hook lifecycle event to execute: pre-commit or pre-push" },
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--fail-on <SEVERITY>", desc: "Minimum finding severity to fail the hook (critical|high|medium|low|info)" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge hook run pre-commit",
        "eforge hook run pre-push --fail-on critical",
      ],
      exitCodes: [
        { code: "0", desc: "Staged/unpushed changes passed verification" },
        { code: "1", desc: "Findings exceeded fail_on threshold; commit/push blocked" },
      ],
    },
    {
      name: "eforge hook status",
      category: "Governance",
      summary: "Inspect the installation, backup preservation, and configuration status of Git hooks in the repository.",
      usage: "eforge hook status [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge hook status",
        "eforge hook status --format json",
      ],
      exitCodes: [
        { code: "0", desc: "Hook status report displayed" },
      ],
    },
    {
      name: "eforge hook uninstall",
      category: "Governance",
      summary: "Remove ELCODE Forge Git hook scripts from .git/hooks/ and automatically restore any .forge-backup files.",
      usage: "eforge hook uninstall [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--hook <TYPE>", desc: "Hook type to uninstall: pre-commit, pre-push, all", default: "all" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge hook uninstall",
        "eforge hook uninstall --hook pre-push",
      ],
      exitCodes: [
        { code: "0", desc: "Hooks removed and backups restored" },
      ],
    },
    {
      name: "eforge policy check",
      category: "Governance",
      summary: "Assert enterprise Policy-as-Code rules, minimum health/dimension scores, secret zero-tolerance, license restrictions, circular dependency caps, and time-bounded waivers.",
      usage: "eforge policy [check] [OPTIONS]",
      flags: [
        { flag: "--policy <PATH>", desc: "Optional path to an external policy file (.yaml or .json); defaults to inline policy: in eforge.config.yaml" },
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--fail-on <SEVERITY>", desc: "Minimum finding severity to fail on (critical|high|medium|low|info)" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge policy check",
        "eforge policy check --policy ./policies/enterprise-gate.yaml --format markdown",
      ],
      exitCodes: [
        { code: "0", desc: "PolicyVerdict::Passed — all constraints satisfied" },
        { code: "1", desc: "PolicyVerdict::Warning — warning threshold reached" },
        { code: "2", desc: "ConfigurationError — no policy defined or invalid YAML/JSON" },
        { code: "4", desc: "SecurityPolicyFailure — one or more policy constraints violated" },
      ],
    },
    {
      name: "eforge audit",
      category: "Compliance",
      summary: "Export automated compliance audit reports mapping repository findings, SBOM dependencies, and architecture controls to SOC 2 Type II, OWASP Top 10 (2021), or ISO/IEC 27001:2022.",
      usage: "eforge audit [OPTIONS]",
      flags: [
        { flag: "--standard <STD>", desc: "Compliance framework standard: soc2, owasp, iso27001", default: "soc2" },
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown", default: "terminal" },
        { flag: "-o, --output <PATH>", desc: "Optional file path to write the rendered audit report" },
      ],
      examples: [
        "eforge audit --standard soc2",
        "eforge audit --standard owasp --format markdown -o owasp-audit.md",
        "eforge audit --standard iso27001 --format json -o iso27001.json",
      ],
      exitCodes: [
        { code: "0", desc: "All evaluated compliance clauses passed" },
        { code: "4", desc: "SecurityPolicyFailure: one or more compliance clauses failed" },
      ],
    },
    {
      name: "eforge report",
      category: "Integration",
      summary: "Generate a deterministic, timestamp-free ReportBundle JSON for Web Intelligence Dashboard consumption and optionally save a commit history snapshot.",
      usage: "eforge report [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "-o, --output <PATH>", desc: "Optional file path to write the rendered ReportBundle JSON" },
        { flag: "--hotspots <N>", desc: "Top N Git churn hotspots to include", default: "10" },
        { flag: "--no-git", desc: "Skip Git history and working-tree diff inspection", default: "false" },
        { flag: "--save-history", desc: "Persist an idempotent history snapshot to .eforge/history/<ts>-<sha12>.json", default: "false" },
      ],
      examples: [
        "eforge report -o forge-report.json",
        "eforge report --save-history -o forge-report.json",
      ],
      exitCodes: [
        { code: "0", desc: "Deterministic ReportBundle JSON emitted" },
      ],
    },
    {
      name: "eforge serve",
      category: "Integration",
      summary: "Start the read-only local loopback HTTP server (127.0.0.1:<port>) exposing /api/v1/health, /api/v1/bundle, and /api/v1/history for live dashboard intelligence.",
      usage: "eforge serve [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--port <PORT>", desc: "Loopback TCP port to bind (127.0.0.1:<port>)", default: "7878" },
        { flag: "--allow-origin <ORIGIN>", desc: "Explicit allowed CORS origin (wildcard '*' is strictly forbidden)" },
        { flag: "--static-dir <DIR>", desc: "Optional directory from which to serve compiled static dashboard assets" },
      ],
      examples: [
        "eforge serve --port 7878",
        "eforge serve --port 7878 --allow-origin http://localhost:5173",
      ],
      exitCodes: [
        { code: "0", desc: "Server exited cleanly" },
      ],
    },
    {
      name: "eforge schema",
      category: "Integration",
      summary: "Export the canonical JSON Schema for ReportBundle and IR types (single source of truth for @elcode-forge/schema and dashboard validation).",
      usage: "eforge schema [OPTIONS]",
      flags: [
        { flag: "-o, --output <PATH>", desc: "Optional file path to write the rendered JSON Schema" },
      ],
      examples: [
        "eforge schema -o packages/forge-schema/report-bundle.schema.json",
      ],
      exitCodes: [
        { code: "0", desc: "JSON Schema rendered deterministically" },
      ],
    },
    {
      name: "eforge mcp",
      category: "Integration",
      summary: "Start the native Model Context Protocol (MCP) JSON-RPC 2.0 stdio server exposing 7 deterministic eforge_* tools wrapped in UntrustedEnvelope<T>.",
      usage: "eforge mcp [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
      ],
      examples: [
        "eforge mcp --path /path/to/repo",
      ],
      exitCodes: [
        { code: "0", desc: "MCP stdio server closed cleanly on EOF" },
      ],
    },
    {
      name: "eforge login",
      category: "Integration",
      summary: "Authenticate with a remote or self-hosted ELCODE Forge Server, validate /api/v1/health, and store credentials in ~/.eforge/credentials.yaml (0o600).",
      usage: "eforge login [OPTIONS]",
      flags: [
        { flag: "--server <URL>", desc: "Remote server URL", default: "https://forge.elcode.id" },
        { flag: "--token <TOKEN>", desc: "Personal Access Token or Service Account Key" },
        { flag: "--username <USER>", desc: "Optional username or service account identifier" },
        { flag: "--no-validate", desc: "Skip remote server /api/v1/health validation check", default: "false" },
      ],
      examples: [
        "eforge login --server http://localhost:8080 --token ef_admin_key",
      ],
      exitCodes: [
        { code: "0", desc: "Credentials verified and saved to ~/.eforge/credentials.yaml" },
      ],
    },
    {
      name: "eforge logout",
      category: "Integration",
      summary: "Remove stored credentials for a specified or default remote ELCODE Forge Server.",
      usage: "eforge logout [OPTIONS]",
      flags: [
        { flag: "--server <URL>", desc: "Remote server URL to log out from (or default server if omitted)" },
      ],
      examples: [
        "eforge logout",
        "eforge logout --server http://localhost:8080",
      ],
      exitCodes: [
        { code: "0", desc: "Stored credentials removed" },
      ],
    },
    {
      name: "eforge auth status",
      category: "Integration",
      summary: "Display current remote authentication status, active server, masked token, and credential store source (also supports eforge auth login / logout).",
      usage: "eforge auth status [OPTIONS]",
      flags: [
        { flag: "--server <URL>", desc: "Remote server URL to inspect (or default server if omitted)" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json", default: "terminal" },
      ],
      examples: [
        "eforge auth status",
        "eforge auth status --format json",
      ],
      exitCodes: [
        { code: "0", desc: "Authentication status displayed with masked token" },
      ],
    },
    {
      name: "eforge push",
      category: "Integration",
      summary: "Upload the local repository analysis snapshot (SyncPushPayload) to a remote Forge Server (POST /api/v1/sync/push) and update the local cached baseline.",
      usage: "eforge push [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--server <URL>", desc: "Remote server URL override" },
        { flag: "--token <TOKEN>", desc: "API token override (defaults to EFORGE_API_TOKEN or stored credentials)" },
        { flag: "--branch <BRANCH>", desc: "Target Git branch name (defaults to active branch or 'main')" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge push",
        "eforge push --branch main --format json",
      ],
      exitCodes: [
        { code: "0", desc: "Snapshot pushed and indexed on Forge Server" },
      ],
    },
    {
      name: "eforge pull",
      category: "Integration",
      summary: "Fetch the remote baseline snapshot from Forge Server (GET /api/v1/sync/pull) and cache it locally in <repo_root>/.eforge/baseline.json.",
      usage: "eforge pull [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--server <URL>", desc: "Remote server URL override" },
        { flag: "--token <TOKEN>", desc: "API token override" },
        { flag: "--branch <BRANCH>", desc: "Target Git branch name (defaults to active branch or 'main')" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge pull",
        "eforge pull --branch develop",
      ],
      exitCodes: [
        { code: "0", desc: "Baseline snapshot saved to .eforge/baseline.json" },
      ],
    },
    {
      name: "eforge sync",
      category: "Integration",
      summary: "Synchronize repository state with a remote Forge Server: pull baseline, compute DeltaSummary (added/resolved findings & health shift), push updated snapshot, and enforce --fail-on.",
      usage: "eforge sync [OPTIONS]",
      flags: [
        { flag: "-p, --path <PATH>", desc: "Target repository directory", default: "." },
        { flag: "--server <URL>", desc: "Remote server URL override" },
        { flag: "--token <TOKEN>", desc: "API token override" },
        { flag: "--branch <BRANCH>", desc: "Target Git branch name" },
        { flag: "--fail-on <SEVERITY>", desc: "Minimum finding severity to fail on (critical|high|medium|low|info)" },
        { flag: "--format <FMT>", desc: "Output format: terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge sync",
        "eforge sync --branch main --fail-on high --format markdown",
      ],
      exitCodes: [
        { code: "0", desc: "Bidirectional sync completed and policy threshold passed" },
        { code: "1", desc: "Findings met or exceeded --fail-on severity" },
      ],
    },
  ];

  let searchQuery = $state("");
  let selectedCategory = $state<string>("All");

  const categories = ["All", "Diagnostics", "Architecture", "Security", "AI", "Governance", "Compliance", "Integration"];

  const filteredCommands = $derived(
    commands.filter((cmd) => {
      const matchesCat = selectedCategory === "All" || cmd.category === selectedCategory;
      const q = searchQuery.toLowerCase();
      const matchesQuery =
        !q ||
        cmd.name.toLowerCase().includes(q) ||
        cmd.summary.toLowerCase().includes(q) ||
        cmd.usage.toLowerCase().includes(q) ||
        cmd.flags.some((f) => f.flag.toLowerCase().includes(q) || f.desc.toLowerCase().includes(q));
      return matchesCat && matchesQuery;
    })
  );
</script>

<div class="space-y-8 max-w-5xl">
  <!-- Header -->
  <div>
    <div class="text-xs font-mono uppercase tracking-wider text-blue-400 mb-1">Command-Line Interface</div>
    <h1 class="text-3xl font-extrabold text-white">CLI Reference & Command Catalog ({commands.length} Commands)</h1>
    <p class="text-slate-400 mt-2 text-base leading-relaxed">
      Complete, authoritative reference for all <code class="text-blue-300 font-mono text-sm">eforge</code> (and alias <code class="text-blue-300 font-mono text-sm">elcode-forge</code>) commands, flags, output formatters, and deterministic exit codes.
    </p>
  </div>

  <!-- Global Exit Codes & Output Formats Contract -->
  <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-5 space-y-3">
      <div class="text-xs font-mono uppercase tracking-wider text-emerald-400 font-bold">Deterministic Exit Codes Contract (0..=4)</div>
      <div class="space-y-1.5 text-xs font-mono">
        <div class="flex items-center justify-between bg-black/40 px-3 py-1.5 rounded border border-slate-800">
          <span class="text-emerald-400 font-bold">0 • Success</span>
          <span class="text-slate-400 font-sans">Execution succeeded; zero policy violations</span>
        </div>
        <div class="flex items-center justify-between bg-black/40 px-3 py-1.5 rounded border border-slate-800">
          <span class="text-amber-400 font-bold">1 • FindingsAboveThreshold</span>
          <span class="text-slate-400 font-sans">Findings reached or exceeded --fail-on severity</span>
        </div>
        <div class="flex items-center justify-between bg-black/40 px-3 py-1.5 rounded border border-slate-800">
          <span class="text-orange-400 font-bold">2 • ConfigurationError</span>
          <span class="text-slate-400 font-sans">Invalid eforge.config.yaml, rule schema, or CLI flag</span>
        </div>
        <div class="flex items-center justify-between bg-black/40 px-3 py-1.5 rounded border border-slate-800">
          <span class="text-rose-400 font-bold">3 • ExecutionError</span>
          <span class="text-slate-400 font-sans">I/O error, path traversal block, or runtime failure</span>
        </div>
        <div class="flex items-center justify-between bg-black/40 px-3 py-1.5 rounded border border-slate-800">
          <span class="text-purple-400 font-bold">4 • SecurityPolicyFailure</span>
          <span class="text-slate-400 font-sans">Security audit, Policy-as-Code, or compliance gate failed</span>
        </div>
      </div>
    </div>

    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-5 space-y-3">
      <div class="text-xs font-mono uppercase tracking-wider text-blue-400 font-bold">Supported --format Output Formatters</div>
      <div class="space-y-1.5 text-xs font-mono">
        <div class="flex items-center justify-between bg-black/40 px-3 py-1.5 rounded border border-slate-800">
          <span class="text-blue-300 font-bold">terminal</span>
          <span class="text-slate-400 font-sans">TTY-aware ANSI theme, visual score meter, ASCII pipe fallback</span>
        </div>
        <div class="flex items-center justify-between bg-black/40 px-3 py-1.5 rounded border border-slate-800">
          <span class="text-blue-300 font-bold">json</span>
          <span class="text-slate-400 font-sans">Byte-for-byte deterministic pretty-printed JSON</span>
        </div>
        <div class="flex items-center justify-between bg-black/40 px-3 py-1.5 rounded border border-slate-800">
          <span class="text-blue-300 font-bold">markdown</span>
          <span class="text-slate-400 font-sans">GitHub-Flavored Markdown tables & Mermaid flowcharts</span>
        </div>
        <div class="flex items-center justify-between bg-black/40 px-3 py-1.5 rounded border border-slate-800">
          <span class="text-blue-300 font-bold">sarif</span>
          <span class="text-slate-400 font-sans">OASIS SARIF v2.1.0 JSON with CVSS v3 security-severity</span>
        </div>
        <div class="flex items-center justify-between bg-black/40 px-3 py-1.5 rounded border border-slate-800">
          <span class="text-blue-300 font-bold">github</span>
          <span class="text-slate-400 font-sans">GitHub Actions workflow annotations (::error, ::warning)</span>
        </div>
      </div>
    </div>
  </div>

  <!-- Search & Filter Controls -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-4 flex flex-col md:flex-row gap-4 items-center justify-between">
    <div class="relative w-full md:w-80">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Search commands, flags, or keywords..."
        class="w-full bg-black/60 border border-slate-800 rounded-lg px-3 py-2 text-xs font-mono text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500 transition-colors"
      />
    </div>

    <div class="flex flex-wrap gap-1.5 w-full md:w-auto">
      {#each categories as cat}
        <button
          onclick={() => (selectedCategory = cat)}
          class="px-2.5 py-1 rounded-md text-xs font-mono transition-colors {selectedCategory === cat ? 'bg-blue-600 text-white font-semibold' : 'bg-slate-900 border border-slate-800 text-slate-400 hover:text-slate-200'}"
        >
          {cat}
        </button>
      {/each}
    </div>
  </div>

  <!-- Command Cards -->
  <div class="space-y-6">
    {#if filteredCommands.length === 0}
      <div class="text-center py-12 text-slate-500 font-mono text-sm">
        No commands match "{searchQuery}" in category "{selectedCategory}".
      </div>
    {/if}

    {#each filteredCommands as cmd}
      <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4 hover:border-slate-700 transition-colors">
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-slate-800/80 pb-3">
          <div class="flex items-center gap-3">
            <h2 class="text-lg font-bold font-mono text-white">{cmd.name}</h2>
            <span class="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-blue-950/80 text-blue-400 border border-blue-800/40">
              {cmd.category}
            </span>
          </div>
          <code class="text-xs font-mono text-slate-400 bg-black/40 px-2 py-1 rounded border border-slate-800 self-start sm:self-auto">
            {cmd.usage}
          </code>
        </div>

        <p class="text-sm text-slate-300 leading-relaxed">{cmd.summary}</p>

        <!-- Flags Table -->
        {#if cmd.flags.length > 0}
          <div class="space-y-1.5">
            <div class="text-xs font-mono font-semibold uppercase tracking-wider text-slate-400">Flags & Arguments</div>
            <div class="overflow-x-auto bg-black/30 border border-slate-800/60 rounded-lg p-2">
              <table class="w-full text-left text-xs font-mono">
                <thead>
                  <tr class="text-slate-500 border-b border-slate-800/60 pb-1">
                    <th class="py-1 px-2 font-medium">Option / Argument</th>
                    <th class="py-1 px-2 font-medium">Description</th>
                    <th class="py-1 px-2 font-medium">Default</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-800/40">
                  {#each cmd.flags as flag}
                    <tr>
                      <td class="py-1.5 px-2 text-blue-300 whitespace-nowrap">{flag.flag}</td>
                      <td class="py-1.5 px-2 text-slate-300 font-sans">{flag.desc}</td>
                      <td class="py-1.5 px-2 text-slate-500">{flag.default ?? "—"}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        {/if}

        <!-- Examples -->
        <div class="space-y-1.5">
          <div class="text-xs font-mono font-semibold uppercase tracking-wider text-slate-400">Practical Examples</div>
          <div class="bg-black/50 border border-slate-800 rounded-lg p-3 font-mono text-xs space-y-1 text-slate-300">
            {#each cmd.examples as ex}
              <div class="flex items-center gap-2">
                <span class="text-blue-500 select-none">$</span>
                <span class="text-slate-100">{ex}</span>
              </div>
            {/each}
          </div>
        </div>

        <!-- Exit Codes -->
        {#if cmd.exitCodes.length > 0}
          <div class="space-y-1.5">
            <div class="text-xs font-mono font-semibold uppercase tracking-wider text-slate-400">Exit Codes Contract</div>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs font-mono">
              {#each cmd.exitCodes as ec}
                <div class="flex items-start gap-2 bg-slate-900/40 border border-slate-800/60 rounded px-2.5 py-1.5">
                  <span class="font-bold {ec.code === '0' ? 'text-emerald-400' : ec.code === '4' ? 'text-purple-400' : 'text-amber-400'}">
                    [{ec.code}]
                  </span>
                  <span class="text-slate-300 font-sans">{ec.desc}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>
