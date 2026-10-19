//! `forge-cli` — Shared CLI execution library for `eforge` and `elcode-forge`.

pub mod cli;
pub mod commands;
pub mod exit_code;
pub mod formatters;

use std::fs;

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
        Some(Commands::Scan {
            path,
            format,
            full,
            incremental,
            fail_on,
            output,
        }) => {
            let opts = commands::scan::ScanOptions {
                repo_root: path,
                incremental: incremental && !full,
                format,
            };
            match commands::scan::execute_scan(&opts) {
                Ok((repo, mut exit_code)) => {
                    if let Some(raw_thresh) = fail_on {
                        if let Some(sev) = parse_severity_threshold(&raw_thresh) {
                            exit_code = ExitCode::evaluate_findings(&repo.findings, Some(sev));
                        } else {
                            eprintln!("Invalid --fail-on severity: {raw_thresh}");
                            return ExitCode::ConfigurationError;
                        }
                    }
                    match render_repository_report(&repo, format) {
                        Ok(rendered) => {
                            if let Some(out_path) = output {
                                if let Err(err) = fs::write(&out_path, &rendered) {
                                    eprintln!(
                                        "Failed to write output to '{}': {err}",
                                        out_path.display()
                                    );
                                    return ExitCode::ExecutionError;
                                }
                            } else {
                                println!("{rendered}");
                            }
                            exit_code
                        }
                        Err(err) => {
                            eprintln!("Error formatting scan report: {err}");
                            ExitCode::ExecutionError
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error running scan: {err}");
                    ExitCode::ExecutionError
                }
            }
        }
        Some(Commands::Review {
            path,
            format,
            fast,
            changed,
            fail_on,
        }) => match commands::review::execute_review(&path, None, fast || changed) {
            Ok((report, mut exit_code)) => {
                if let Some(raw_thresh) = fail_on {
                    if let Some(sev) = parse_severity_threshold(&raw_thresh) {
                        exit_code = ExitCode::evaluate_findings(&report.findings, Some(sev));
                    } else {
                        eprintln!("Invalid --fail-on severity: {raw_thresh}");
                        return ExitCode::ConfigurationError;
                    }
                }
                match commands::review::render_review_report(&report, format) {
                    Ok(rendered) => {
                        println!("{rendered}");
                        exit_code
                    }
                    Err(err) => {
                        eprintln!("Error formatting review report: {err}");
                        ExitCode::ExecutionError
                    }
                }
            }
            Err(err) => {
                eprintln!("Error running review: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::Diff { path, format }) => match commands::diff::execute_diff(&path, None) {
            Ok(report) => match commands::diff::render_diff_report(&report, format) {
                Ok(rendered) => {
                    println!("{rendered}");
                    ExitCode::Success
                }
                Err(err) => {
                    eprintln!("Error formatting diff report: {err}");
                    ExitCode::ExecutionError
                }
            },
            Err(err) => {
                eprintln!("Error running diff: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::Dependencies { path, format }) => {
            match commands::dependencies::run_dependencies(&path, format) {
                Ok(rendered) => {
                    println!("{rendered}");
                    ExitCode::Success
                }
                Err(err) => {
                    eprintln!("Error running dependencies: {err}");
                    ExitCode::ExecutionError
                }
            }
        }
        Some(Commands::Security { path, format }) => {
            match commands::security::run_security(&path, format) {
                Ok((rendered, exit_code)) => {
                    println!("{rendered}");
                    exit_code
                }
                Err(err) => {
                    eprintln!("Error running security scan: {err}");
                    ExitCode::ExecutionError
                }
            }
        }
        Some(Commands::Graph { path, format }) => match commands::graph::run_graph(&path, format) {
            Ok(rendered) => {
                println!("{rendered}");
                ExitCode::Success
            }
            Err(err) => {
                eprintln!("Error running graph: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::Onboard { path, output } | Commands::Docs { path, output }) => {
            let out_dir = output.unwrap_or_else(|| path.join("docs").join("forge"));
            match commands::onboard::generate_onboarding_docs(&path, &out_dir) {
                Ok(bundle) => {
                    println!(
                        "Generated {} documentation file(s) in {}",
                        bundle.files_written.len(),
                        bundle.output_dir.display()
                    );
                    ExitCode::Success
                }
                Err(err) => {
                    eprintln!("Error generating documentation: {err}");
                    ExitCode::ExecutionError
                }
            }
        }
        Some(Commands::Mcp { path }) => match commands::mcp::run_mcp(&path) {
            Ok(()) => ExitCode::Success,
            Err(err) => {
                eprintln!("Error running MCP server: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::Explain {
            target,
            path,
            format,
        }) => match commands::explain::execute_explain(&path, target.as_deref()) {
            Ok(report) => match commands::explain::render_explain_report(&report, format) {
                Ok(rendered) => {
                    println!("{rendered}");
                    ExitCode::Success
                }
                Err(err) => {
                    eprintln!("Error formatting explain report: {err}");
                    ExitCode::ExecutionError
                }
            },
            Err(err) => {
                eprintln!("Error running explain: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::Report {
            path,
            output,
            hotspots,
            no_git,
        }) => match commands::report::run_report(&path, output.as_deref(), hotspots, no_git) {
            Ok(rendered) => {
                if output.is_none() {
                    println!("{rendered}");
                }
                ExitCode::Success
            }
            Err(err) => {
                eprintln!("Error generating report bundle: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::Schema { output }) => {
            match commands::schema::run_schema(output.as_deref()) {
                Ok(rendered) => {
                    if output.is_none() {
                        println!("{rendered}");
                    }
                    ExitCode::Success
                }
                Err(err) => {
                    eprintln!("Error exporting schema: {err}");
                    ExitCode::ExecutionError
                }
            }
        }
    }
}
