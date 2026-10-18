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

/// Top-level `eforge` subcommands for Phase 0 & Phase 1.
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
}
