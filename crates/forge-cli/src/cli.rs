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
    /// Inspect working-tree unified diff and affected architecture modules.
    Diff {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
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
    /// Run focused secret detection and unsafe code pattern security analysis.
    Security {
        /// Target repository directory.
        #[arg(long, short = 'p', default_value = ".")]
        path: PathBuf,
        /// Output format (`terminal`, `json`, `markdown`, `sarif`, `github`).
        #[arg(long, value_enum, default_value_t = OutputFormat::Terminal)]
        format: OutputFormat,
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
}
