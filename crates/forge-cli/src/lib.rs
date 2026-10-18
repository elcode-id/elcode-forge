//! `forge-cli` — Shared CLI execution library for `eforge` and `elcode-forge`.

pub mod cli;
pub mod commands;
pub mod exit_code;
pub mod formatters;

use clap::Parser;
pub use cli::{Cli, Commands, OutputFormat};
pub use exit_code::{parse_severity_threshold, ExitCode};
pub use formatters::render_repository_report;

/// Returns the formatted version banner for the CLI.
pub fn version_banner() -> String {
    format!("ELCODE Forge (eforge) v{}", env!("CARGO_PKG_VERSION"))
}

/// Parses CLI arguments and executes the selected subcommand, returning a deterministic [`ExitCode`].
pub fn run_cli_from_env() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        None => {
            println!("{}", version_banner());
            ExitCode::Success
        }
        Some(Commands::Init { path, .. }) => match commands::init::run_init(&path) {
            Ok(cfg_path) => {
                println!("Initialized ELCODE Forge at {}", cfg_path.display());
                ExitCode::Success
            }
            Err(err) => {
                eprintln!("Error: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::Doctor { path, format }) => match commands::doctor::run_doctor(&path) {
            Ok(report) => match commands::doctor::render_doctor_report(&report, format) {
                Ok(rendered) => {
                    println!("{rendered}");
                    ExitCode::Success
                }
                Err(err) => {
                    eprintln!("Error formatting doctor report: {err}");
                    ExitCode::ExecutionError
                }
            },
            Err(err) => {
                eprintln!("Error running doctor: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::Scan { .. } | Commands::Review { .. } | Commands::Diff { .. }) => {
            println!("{}", version_banner());
            ExitCode::Success
        }
    }
}
