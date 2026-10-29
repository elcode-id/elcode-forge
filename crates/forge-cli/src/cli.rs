//! `clap` command-line argument definitions for `eforge` and `elcode-forge`.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

/// Supported output formats for `eforge` reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Human-readable terminal summary.
    Terminal,
    /// Pretty-printed JSON report.
    Json,
    /// GitHub-flavored Markdown report.
    Markdown,
    /// OASIS SARIF v2.1.0 JSON report.
    Sarif,
    /// GitHub Actions workflow annotations + Markdown summary.
    Github,
}

/// Supported Software Bill of Materials (SBOM) formats for `eforge sbom`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CliSbomFormat {
    /// OASIS / OWASP CycloneDX v1.5 JSON
    #[value(name = "cyclonedx")]
    CycloneDx,
    /// ISO/IEC 5962:2021 SPDX v2.3 JSON
    #[value(name = "spdx")]
    Spdx,
}

impl From<CliSbomFormat> for forge_security::SbomFormat {
    fn from(format: CliSbomFormat) -> Self {
        match format {
            CliSbomFormat::CycloneDx => forge_security::SbomFormat::CycloneDx,
            CliSbomFormat::Spdx => forge_security::SbomFormat::Spdx,
        }
    }
}

/// Primary CLI parser for `eforge` / `elcode-forge`.
#[derive(Debug, Parser)]
#[command(
    name = "eforge",
    version,
    about = "ELCODE Forge — Codebase Intelligence, Architecture & Release Readiness Engine",
    long_about = None
)]
pub struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Top-level `eforge` subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize `eforge.config.yaml` and `.eforge/` workspace metadata.
    Init {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Overwrite existing `eforge.config.yaml` if present.
        #[arg(long)]
        force: bool,
    },
    /// Run a fast (<5s) repository diagnostic check (`eforge doctor`).
    Doctor {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output format (`terminal`, `json`, `markdown`, `sarif`, `github`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Execute a full or incremental architecture, dependency, rule, and security scan.
    Scan {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output format (`terminal`, `json`, `markdown`, `sarif`, `github`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
        /// Force a full repository scan.
        #[arg(long)]
        full: bool,
        /// Use `.eforge/` BLAKE3 content hashes to inspect changed files.
        #[arg(long)]
        incremental: bool,
        /// Minimum finding severity to return non-zero exit code (`critical|high|medium|low|info`).
        #[arg(long)]
        fail_on: Option<String>,
        /// Optional file path to write the rendered report.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Analyze working-tree diff blast radius, change risk, and findings on modified files.
    Review {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output format (`terminal`, `json`, `markdown`, `sarif`, `github`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
        /// Run fast diff-scoped mode.
        #[arg(long)]
        fast: bool,
        /// Scope analysis to changed files in working tree.
        #[arg(long)]
        changed: bool,
        /// Minimum finding severity to return non-zero exit code (`critical|high|medium|low|info`).
        #[arg(long)]
        fail_on: Option<String>,
    },
    /// Analyze changes with deterministic diff/PR review augmented by structured AI review items.
    #[command(name = "ai-review")]
    AiReview {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Base Git reference for diff comparison (e.g. `origin/main` or `HEAD~1`).
        #[arg(long)]
        base: Option<String>,
        /// Head Git reference for diff comparison (defaults to working tree).
        #[arg(long)]
        head: Option<String>,
        /// Run fast diff-scoped mode.
        #[arg(long)]
        fast: bool,
        /// Minimum finding severity to return non-zero exit code (`critical|high|medium|low|info`).
        #[arg(long)]
        fail_on: Option<String>,
        /// AI provider override (`openai`, `anthropic`, `gemini`, `mock`).
        #[arg(long)]
        provider: Option<String>,
        /// Model identifier override.
        #[arg(long)]
        model: Option<String>,
        /// Output context pack and token estimates without calling AI provider.
        #[arg(long)]
        dry_run: bool,
        /// Bypass AI response cache.
        #[arg(long)]
        no_cache: bool,
        /// Output format (`terminal`, `json`, `markdown`, `sarif`, `github`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Inspect working-tree unified diff and affected architecture modules.
    Diff {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Compare findings and health against remote baseline snapshot.
        #[arg(long)]
        remote: bool,
        /// Output format (`terminal`, `json`, `markdown`, `sarif`, `github`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Inspect package manifest dependencies, unused packages, and version hygiene.
    Dependencies {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output format (`terminal`, `json`, `markdown`, `sarif`, `github`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Export Software Bill of Materials (SBOM) in CycloneDX v1.5 or SPDX v2.3 JSON format.
    Sbom {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// SBOM format (`cyclonedx`, `spdx`).
        #[arg(long, value_enum, default_value_t = CliSbomFormat::CycloneDx)]
        format: CliSbomFormat,
        /// Optional file path to write the rendered SBOM JSON.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Run focused secret detection, unsafe code pattern analysis, and OSV dependency vulnerability audit.
    Security {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output format (`terminal`, `json`, `markdown`, `sarif`, `github`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
        /// Run deep dependency vulnerability audit against OSV advisories.
        #[arg(long)]
        audit: bool,
        /// Run in offline mode without remote network queries.
        #[arg(long)]
        offline: bool,
        /// Minimum finding severity to return non-zero exit code (`critical|high|medium|low|info`).
        #[arg(long)]
        fail_on: Option<String>,
    },
    /// Export the inferred module architecture graph (Terminal, JSON, or Mermaid Markdown).
    Graph {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output format (`terminal`, `json`, `markdown`, `sarif`, `github`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Generate developer onboarding and architecture documentation (`docs/forge/`).
    Onboard {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output directory for generated Markdown docs (defaults to `<path>/docs/forge`).
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Generate deterministic architecture, module, dependency, and risk documentation (`docs/forge/`).
    Docs {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output directory for generated Markdown docs (defaults to `<path>/docs/forge`).
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Start the native Model Context Protocol (MCP) JSON-RPC 2.0 stdio server (`eforge_*` tools).
    Mcp {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
    },
    /// Explain the structural role, files, imports, dependents, exports, and findings for a project or module.
    Explain {
        /// Optional module ID or file path prefix (defaults to "." for full project overview).
        target: Option<String>,
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output format (`terminal`, `json`, `markdown`, `sarif`, `github`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
        /// Augment structural explanation with AI interpretation.
        #[arg(long)]
        ai: bool,
        /// AI provider override (`openai`, `anthropic`, `gemini`, `mock`).
        #[arg(long)]
        provider: Option<String>,
        /// Model identifier override.
        #[arg(long)]
        model: Option<String>,
        /// Output context pack and token estimates without calling AI provider.
        #[arg(long)]
        dry_run: bool,
        /// Bypass AI response cache.
        #[arg(long)]
        no_cache: bool,
    },
    /// Generate a deterministic ReportBundle JSON for dashboard consumption.
    Report {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Optional file path to write the rendered ReportBundle JSON.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
        /// Top N churn hotspots to include.
        #[arg(long, default_value_t = 10)]
        hotspots: usize,
        /// Skip Git history and working tree diff inspection.
        #[arg(long)]
        no_git: bool,
        /// Persist a deterministic history snapshot to .eforge/history/ for trends analysis.
        #[arg(long)]
        save_history: bool,
    },
    /// Export the JSON Schema for ReportBundle (single source of truth for dashboard).
    Schema {
        /// Optional file path to write the rendered JSON Schema.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Start the read-only local loopback HTTP server for dashboard live intelligence.
    Serve {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Port to bind the HTTP server to (127.0.0.1:<port>).
        #[arg(long, default_value_t = 7878)]
        port: u16,
        /// Allowed origin for CORS (explicit origin only, wildcard forbidden).
        #[arg(long)]
        allow_origin: Option<String>,
        /// Optional directory from which to serve static dashboard assets.
        #[arg(long)]
        static_dir: Option<PathBuf>,
    },
    /// Manage architectural, quality, and security rules (`list`, `check`, `init`).
    Rules {
        #[command(subcommand)]
        action: Option<RulesAction>,
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Manage native Git hooks automation (`install`, `run`, `uninstall`, `status`).
    Hook {
        #[command(subcommand)]
        action: Option<HookAction>,
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Manage AI configuration, check provider status, and inspect usage ledgers.
    Ai {
        #[command(subcommand)]
        action: AiAction,
    },
    /// Authenticate with a remote ELCODE Forge Server.
    Login {
        /// Remote server URL (defaults to https://forge.elcode.id).
        #[arg(long, default_value = "https://forge.elcode.id")]
        server: String,
        /// Personal Access Token or Service Account Key.
        #[arg(long)]
        token: Option<String>,
        /// Optional username or service account identifier.
        #[arg(long)]
        username: Option<String>,
        /// Skip server health validation check.
        #[arg(long)]
        no_validate: bool,
    },
    /// Remove stored credentials for a remote ELCODE Forge Server.
    Logout {
        /// Remote server URL to log out from (or default server if omitted).
        #[arg(long)]
        server: Option<String>,
    },
    /// Manage remote authentication and credentials.
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Upload local analysis snapshot to remote Forge Server.
    Push {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Remote server URL override (defaults to configured server).
        #[arg(long)]
        server: Option<String>,
        /// API token override (defaults to EFORGE_API_TOKEN or stored credentials).
        #[arg(long)]
        token: Option<String>,
        /// Target Git branch name (defaults to active branch or "main").
        #[arg(long)]
        branch: Option<String>,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Fetch remote baseline snapshot from Forge Server and cache locally.
    Pull {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Remote server URL override (defaults to configured server).
        #[arg(long)]
        server: Option<String>,
        /// API token override (defaults to EFORGE_API_TOKEN or stored credentials).
        #[arg(long)]
        token: Option<String>,
        /// Target Git branch name (defaults to active branch or "main").
        #[arg(long)]
        branch: Option<String>,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Synchronize repository state with remote Forge Server (evaluate, compare baseline, push).
    Sync {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Remote server URL override (defaults to configured server).
        #[arg(long)]
        server: Option<String>,
        /// API token override (defaults to EFORGE_API_TOKEN or stored credentials).
        #[arg(long)]
        token: Option<String>,
        /// Target Git branch name (defaults to active branch or "main").
        #[arg(long)]
        branch: Option<String>,
        /// Minimum finding severity to fail on (`critical|high|medium|low|info`).
        #[arg(long)]
        fail_on: Option<String>,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Assert enterprise Policy-as-Code rules, constraints, and thresholds (`eforge policy check`).
    Policy {
        #[command(subcommand)]
        action: Option<PolicyAction>,
        /// Optional path to an external policy file (`.yaml` or `.json`).
        #[arg(long)]
        policy: Option<PathBuf>,
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Minimum finding severity to fail on (`critical|high|medium|low|info`).
        #[arg(long)]
        fail_on: Option<String>,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
}

/// Subcommands available under `eforge policy`.
#[derive(Debug, Subcommand)]
pub enum PolicyAction {
    /// Evaluate repository findings, scores, and change impact against organizational policy.
    Check {
        /// Optional path to an external policy file (`.yaml` or `.json`).
        #[arg(long)]
        policy: Option<PathBuf>,
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Minimum finding severity to fail on (`critical|high|medium|low|info`).
        #[arg(long)]
        fail_on: Option<String>,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
}

/// Actions available under `eforge auth`.
#[derive(Debug, Subcommand)]
pub enum AuthAction {
    /// Authenticate with a remote ELCODE Forge Server.
    Login {
        /// Remote server URL (defaults to https://forge.elcode.id).
        #[arg(long, default_value = "https://forge.elcode.id")]
        server: String,
        /// Personal Access Token or Service Account Key.
        #[arg(long)]
        token: Option<String>,
        /// Optional username or service account identifier.
        #[arg(long)]
        username: Option<String>,
        /// Skip server health validation check.
        #[arg(long)]
        no_validate: bool,
    },
    /// Remove stored credentials for a remote ELCODE Forge Server.
    Logout {
        /// Remote server URL to log out from (or default server if omitted).
        #[arg(long)]
        server: Option<String>,
    },
    /// Display current authentication and token status.
    Status {
        /// Remote server URL to inspect (or default server if omitted).
        #[arg(long)]
        server: Option<String>,
        /// Output format (`terminal`, `json`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
}

/// Actions available under `eforge ai`.
#[derive(Debug, Subcommand)]
pub enum AiAction {
    /// Inspect effective AI configuration, credentials status, and connectivity.
    Status {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Test live provider reachability and measure round-trip latency.
        #[arg(long)]
        ping: bool,
        /// Output format (`terminal`, `json`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Summarize AI token consumption, response cache hits, and daily spend.
    Usage {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output format (`terminal`, `json`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
}

/// Subcommands available under `eforge rules`.
#[derive(Debug, Subcommand)]
pub enum RulesAction {
    /// List all registered rules, their pack source, severity, and status.
    List {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Filter by rule category (`architecture`, `quality`, `security`, `dependencies`).
        #[arg(long, short = 'c')]
        category: Option<String>,
        /// Show only enabled rules.
        #[arg(long)]
        enabled_only: bool,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Validate syntax of rule files without scanning.
    Check {
        /// Optional path to a specific rule file or directory (defaults to configured paths and `.eforge/rules`).
        path: Option<PathBuf>,
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        repo: PathBuf,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Generate a starter rule template in `.eforge/rules/custom-rules.yaml`.
    Init {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output file path (defaults to `<path>/.eforge/rules/custom-rules.yaml`).
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
        /// Force overwrite if destination file already exists.
        #[arg(long)]
        force: bool,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
}

/// Subcommands available under `eforge hook`.
#[derive(Debug, Subcommand)]
pub enum HookAction {
    /// Install native Git hook scripts into `.git/hooks/`.
    Install {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Hook type to install (`pre-commit`, `pre-push`, `all`).
        #[arg(long, default_value = "all")]
        hook: String,
        /// Force overwrite of existing hook files.
        #[arg(long)]
        force: bool,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Execute a hook lifecycle check (called by git or manual invocation).
    Run {
        /// Hook event being executed (`pre-commit`, `pre-push`).
        hook: String,
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Minimum finding severity to fail the hook (`critical|high|medium|low|info`).
        #[arg(long)]
        fail_on: Option<String>,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Remove managed Git hook scripts and restore backups.
    Uninstall {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Hook type to uninstall (`pre-commit`, `pre-push`, `all`).
        #[arg(long, default_value = "all")]
        hook: String,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
    /// Inspect the installation and configuration status of Git hooks.
    Status {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output format (`terminal`, `json`, `markdown`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
    },
}
