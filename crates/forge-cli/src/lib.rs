//! `forge-cli` — Shared CLI execution library for `eforge` and `elcode-forge`.

pub mod cli;
pub mod exit_code;
pub mod formatters;

pub use cli::{Cli, Commands, OutputFormat};
pub use exit_code::{parse_severity_threshold, ExitCode};
pub use formatters::render_repository_report;

/// Returns the formatted version banner for the CLI.
pub fn version_banner() -> String {
    format!("ELCODE Forge (eforge) v{}", env!("CARGO_PKG_VERSION"))
}
