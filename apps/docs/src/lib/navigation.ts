export interface NavLink {
  title: string;
  href: string;
  exact?: boolean;
}

export interface NavSection {
  title: string;
  links: NavLink[];
}

export interface SearchEntry {
  title: string;
  pageTitle: string;
  sectionTitle: string;
  href: string;
  keywords: string;
}

export const navigation: NavSection[] = [
  {
    title: "Introduction",
    links: [
      { title: "Getting started", href: "/", exact: true },
      { title: "Installation", href: "/#installation" },
      { title: "Golden Path quickstart", href: "/#quickstart" },
      { title: "Language & ecosystem matrix", href: "/#ecosystems" },
      { title: "8-Dimension Health Score", href: "/#health-score" },
    ],
  },
  {
    title: "Core concepts",
    links: [
      { title: "Configuration (eforge.config.yaml)", href: "/configuration", exact: true },
      { title: "AI providers & credential security", href: "/configuration#ai-providers" },
      { title: "Environment variables", href: "/configuration#env-vars" },
      { title: "GitHub Action (forge-action)", href: "/configuration#github-action" },
    ],
  },
  {
    title: "CLI & Rule Engine",
    links: [
      { title: "CLI command reference", href: "/cli", exact: true },
      { title: "Output formats & exit codes", href: "/cli#exit-codes" },
      { title: "Custom rules engine", href: "/rules", exact: true },
      { title: "6 deterministic matchers", href: "/rules#matchers" },
      { title: "Topology inference & auto-fix", href: "/rules#inference" },
    ],
  },
  {
    title: "AI & Governance",
    links: [
      { title: "MCP server (eforge_*)", href: "/mcp", exact: true },
      { title: "Prompt-injection defense", href: "/mcp#security-envelope" },
      { title: "Policy-as-Code gates", href: "/policy", exact: true },
      { title: "Security, OSV & SBOM", href: "/compliance", exact: true },
    ],
  },
  {
    title: "Enterprise & Supply Chain",
    links: [
      { title: "Self-hosting (forge-server)", href: "/self-hosting", exact: true },
      { title: "SLSA Level 3 provenance", href: "/provenance", exact: true },
    ],
  },
];

export const primaryPages: { title: string; section: string; href: string; description: string }[] = [
  {
    title: "Getting started",
    section: "Introduction",
    href: "/",
    description:
      "Install ELCODE Forge (eforge v1.1.0), run your first deterministic codebase scan in under 5 seconds, and explore the interactive TUI and 8-Dimension Health Score.",
  },
  {
    title: "Configuration reference",
    section: "Core concepts",
    href: "/configuration",
    description:
      "Complete schema reference for eforge.config.yaml, AI provider routing, environment variable isolation, .eforge/ cache layout, and GitHub Action inputs.",
  },
  {
    title: "CLI command reference",
    section: "CLI & Rule Engine",
    href: "/cli",
    description:
      "Exhaustive syntax guide for all 41 eforge CLI commands and subcommands (including ui, watch, fix, simulate, trend, blame-arch, and rules infer), 5 output formatters, and deterministic exit codes (0..=4).",
  },
  {
    title: "Custom rules engine",
    section: "CLI & Rule Engine",
    href: "/rules",
    description:
      "Author deterministic YAML/JSON architectural rules using 4 built-in packs, 6 AST/import/filesystem matchers, automatic topology boundary inference (eforge rules infer), and transactional auto-fix (eforge fix).",
  },
  {
    title: "Native MCP server",
    section: "AI & Governance",
    href: "/mcp",
    description:
      "Connect Claude Code, Cursor, Windsurf, and VS Code to eforge mcp over stdio JSON-RPC 2.0 with 11 eforge_* tools (including closed-loop pre-flight patch verification) and UntrustedEnvelope isolation.",
  },
  {
    title: "Policy-as-Code gates",
    section: "AI & Governance",
    href: "/policy",
    description:
      "Enforce organization-wide health scores, blast-radius thresholds, time-bounded waivers, and severity escalations in CI/CD pipelines.",
  },
  {
    title: "Security, OSV, SBOM & compliance",
    section: "AI & Governance",
    href: "/compliance",
    description:
      "Generate SOC2, OWASP Top 10, and ISO 27001 audit evidence, run OSV vulnerability checks, and export CycloneDX v1.5 & SPDX v2.3 SBOMs.",
  },
  {
    title: "Self-hosting (forge-server)",
    section: "Enterprise & Supply Chain",
    href: "/self-hosting",
    description:
      "Deploy the multi-tenant Axum + SQLite WAL governance backend via Docker, Compose, or Kubernetes with RBAC, SSO/JWKS, and HMAC-SHA256 webhooks.",
  },
  {
    title: "SLSA Level 3 provenance",
    section: "Enterprise & Supply Chain",
    href: "/provenance",
    description:
      "Cryptographically verify eforge and elcode-forge release binaries using GitHub Artifact Attestations, Sigstore Cosign, and SHA256SUMS.txt.",
  },
];

export const searchIndex: SearchEntry[] = [
  {
    title: "Getting started with ELCODE Forge (eforge v1.1.0)",
    pageTitle: "Getting started",
    sectionTitle: "Introduction",
    href: "/",
    keywords: "overview manifesto quickstart eforge elcode-forge rust cli deterministic v1.1.0",
  },
  {
    title: "Installation (npx, curl install.sh, PowerShell, Homebrew, Cargo)",
    pageTitle: "Getting started",
    sectionTitle: "Introduction",
    href: "/#installation",
    keywords: "install npx elcode-forge brew tap cargo binstall curl powershell windows macos linux",
  },
  {
    title: "Golden Path Quickstart (init, doctor, ui, watch, scan, fix, review, report)",
    pageTitle: "Getting started",
    sectionTitle: "Introduction",
    href: "/#quickstart",
    keywords: "quickstart init doctor ui watch scan fix simulate trend review report sarif html",
  },
  {
    title: "Multi-Language & Ecosystem Support Matrix",
    pageTitle: "Getting started",
    sectionTitle: "Introduction",
    href: "/#ecosystems",
    keywords: "typescript javascript python go rust php svelte vue nextjs laravel cargo",
  },
  {
    title: "8-Dimension Health Score & Risk Formula",
    pageTitle: "Getting started",
    sectionTitle: "Introduction",
    href: "/#health-score",
    keywords: "health score architecture security maintainability reliability testing dependency documentation release",
  },
  {
    title: "eforge.config.yaml & forge.config.yaml Schema",
    pageTitle: "Configuration reference",
    sectionTitle: "Core concepts",
    href: "/configuration",
    keywords: "config yaml ignore rulesetextends policy ai cache index",
  },
  {
    title: "AI Providers (Ollama, LM Studio, OpenAI, Anthropic, Gemini) & api_key_env",
    pageTitle: "Configuration reference",
    sectionTitle: "Core concepts",
    href: "/configuration#ai-providers",
    keywords: "ai ollama lmstudio openai anthropic gemini api_key_env max_context_tokens redact_secrets",
  },
  {
    title: "Environment Variables (EFORGE_AI_API_KEY, EFORGE_SERVER_*, NO_COLOR)",
    pageTitle: "Configuration reference",
    sectionTitle: "Core concepts",
    href: "/configuration#env-vars",
    keywords: "environment variables EFORGE_AI_API_KEY EFORGE_JWT_SECRET EFORGE_DATABASE_URL",
  },
  {
    title: "GitHub Action (elcode-id/forge-action@v1) Inputs & Outputs",
    pageTitle: "Configuration reference",
    sectionTitle: "Core concepts",
    href: "/configuration#github-action",
    keywords: "github action ci workflow sarif pr-comment fail-on",
  },
  {
    title: "CLI Reference — All 41 eforge Commands & Flags",
    pageTitle: "CLI command reference",
    sectionTitle: "CLI & Rule Engine",
    href: "/cli",
    keywords: "cli doctor scan ui watch fix simulate trend blame-arch review security sbom graph policy audit mcp",
  },
  {
    title: "Interactive Terminal UI (eforge ui & --interactive / --codeframe)",
    pageTitle: "CLI command reference",
    sectionTitle: "CLI & Rule Engine",
    href: "/cli#commands",
    keywords: "eforge ui ratatui interactive tui codeframe dag explorer tarjan scc blast radius",
  },
  {
    title: "Live Architectural Radar (eforge watch) & Surgical Auto-Fix (eforge fix)",
    pageTitle: "CLI command reference",
    sectionTitle: "CLI & Rule Engine",
    href: "/cli#commands",
    keywords: "eforge watch incremental blake3 cache eforge fix dry-run apply rollback transaction",
  },
  {
    title: "In-Memory Refactor Simulator (eforge simulate) & Temporal Graph (eforge trend, eforge blame-arch)",
    pageTitle: "CLI command reference",
    sectionTitle: "CLI & Rule Engine",
    href: "/cli#commands",
    keywords: "eforge simulate move refactor plan eforge trend hotspots eforge blame-arch git blame",
  },
  {
    title: "Output Formats (terminal, json, markdown, sarif, github) & Exit Codes (0..=4)",
    pageTitle: "CLI command reference",
    sectionTitle: "CLI & Rule Engine",
    href: "/cli#exit-codes",
    keywords: "format json sarif markdown github exit code 0 1 2 3 4 ci gate",
  },
  {
    title: "Custom Rules Engine & 4 Built-in Rule Packs",
    pageTitle: "Custom rules engine",
    sectionTitle: "CLI & Rule Engine",
    href: "/rules",
    keywords: "rules forge:recommended forge:strict forge:security forge:clean-architecture inheritance",
  },
  {
    title: "6 Deterministic Rule Matchers (forbidden_imports, required_companion, file_naming, file_size, regex_pattern, dependency_restriction)",
    pageTitle: "Custom rules engine",
    sectionTitle: "CLI & Rule Engine",
    href: "/rules#matchers",
    keywords: "matchers forbidden_imports required_companion file_naming file_size regex_pattern dependency_restriction",
  },
  {
    title: "Topology & Boundary Inference (eforge rules infer & eforge init --infer)",
    pageTitle: "Custom rules engine",
    sectionTitle: "CLI & Rule Engine",
    href: "/rules#inference",
    keywords: "eforge rules infer init --infer topology layer instability fan-in fan-out clean architecture",
  },
  {
    title: "Native MCP Server (eforge mcp) & IDE Setup (Claude, Cursor, Windsurf, VS Code)",
    pageTitle: "Native MCP server",
    sectionTitle: "AI & Governance",
    href: "/mcp",
    keywords: "mcp server stdio claude cursor windsurf vscode json-rpc",
  },
  {
    title: "11 eforge_* MCP Tools (including verify_patch, simulate_refactor, get_impacted_tests, get_pruned_context)",
    pageTitle: "Native MCP server",
    sectionTitle: "AI & Governance",
    href: "/mcp#tools",
    keywords: "eforge_project_info eforge_get_architecture eforge_get_dependencies eforge_get_findings eforge_get_change_impact eforge_explain_module eforge_get_rules eforge_verify_patch eforge_simulate_refactor eforge_get_impacted_tests eforge_get_pruned_context",
  },
  {
    title: "UntrustedEnvelope<T> & 7 Prompt-Injection Detectors",
    pageTitle: "Native MCP server",
    sectionTitle: "AI & Governance",
    href: "/mcp#security-envelope",
    keywords: "prompt injection untrusted envelope sanitization security boundary",
  },
  {
    title: "Policy-as-Code Gates (.eforge/policy.yaml), Constraints & Waivers",
    pageTitle: "Policy-as-Code gates",
    sectionTitle: "AI & Governance",
    href: "/policy",
    keywords: "policy constraints min_health_score max_risk_score max_blast_radius waivers rule_overrides",
  },
  {
    title: "Compliance Frameworks (SOC2, OWASP Top 10, ISO/IEC 27001), OSV & SBOM",
    pageTitle: "Security, OSV, SBOM & compliance",
    sectionTitle: "AI & Governance",
    href: "/compliance",
    keywords: "compliance audit soc2 owasp iso27001 sbom cyclonedx spdx osv vulnerability secrets entropy",
  },
  {
    title: "Self-Hosting forge-server (Docker, Compose, Kubernetes, RBAC, SSO/JWKS, Webhooks)",
    pageTitle: "Self-hosting (forge-server)",
    sectionTitle: "Enterprise & Supply Chain",
    href: "/self-hosting",
    keywords: "self-hosting forge-server docker compose kubernetes rbac jwt jwks webhooks sqlite",
  },
  {
    title: "SLSA Level 3 Provenance, Sigstore Cosign & SHA256SUMS Verification",
    pageTitle: "SLSA Level 3 provenance",
    sectionTitle: "Enterprise & Supply Chain",
    href: "/provenance",
    keywords: "slsa provenance attestation cosign gh attestation verify sha256sums supply chain",
  },
];
