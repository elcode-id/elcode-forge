//! `forge-cli` — Shared CLI execution library for `eforge` and `elcode-forge`.

pub mod ai;
pub mod cli;
pub mod commands;
pub mod exit_code;
pub mod formatters;
pub mod serve;
pub mod theme;
pub mod tui;

use std::fs;
use std::path::Path;

use clap::Parser;
pub use cli::{
    AuthAction, Cli, CliComplianceStandard, Commands, OutputFormat, PolicyAction, SimulateAction,
};
pub use exit_code::{parse_severity_threshold, ExitCode};
pub use formatters::render_repository_report;
pub use theme::{health_rating, render_score_meter, ColorMode, Theme};

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
        Some(Commands::Init { path, force, infer }) => {
            match commands::init::run_init_with_options(&path, force, infer) {
                Ok(cfg_path) => {
                    println!("Initialized ELCODE Forge at {}", cfg_path.display());
                    if infer {
                        println!(
                            "Generated inferred architectural boundary rules at {}",
                            path.join(".eforge")
                                .join("rules")
                                .join("inferred-architecture.yaml")
                                .display()
                        );
                    }
                    ExitCode::Success
                }
                Err(err) => {
                    eprintln!("Error: {err}");
                    ExitCode::ExecutionError
                }
            }
        }
        Some(Commands::Rules {
            action,
            path,
            format,
        }) => {
            match action.unwrap_or(cli::RulesAction::List {
                path: path.clone(),
                category: None,
                enabled_only: false,
                format,
            }) {
                cli::RulesAction::List {
                    path: list_path,
                    category,
                    enabled_only,
                    format: list_format,
                } => {
                    let target_dir = if list_path != Path::new(".") {
                        list_path
                    } else {
                        path
                    };
                    let active_format = if list_format != cli::OutputFormat::Terminal {
                        list_format
                    } else {
                        format
                    };
                    match commands::rules::run_rules_list(
                        &target_dir,
                        category.as_deref(),
                        enabled_only,
                    ) {
                        Ok(report) => {
                            match commands::rules::render_rules_list(&report, active_format) {
                                Ok(rendered) => {
                                    println!("{rendered}");
                                    ExitCode::Success
                                }
                                Err(err) => {
                                    eprintln!("Error formatting rules list: {err}");
                                    ExitCode::ExecutionError
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error listing rules: {err}");
                            ExitCode::ExecutionError
                        }
                    }
                }
                cli::RulesAction::Check {
                    path: target_file,
                    repo,
                    format: check_format,
                } => {
                    let active_repo = if repo != Path::new(".") { repo } else { path };
                    let active_format = if check_format != cli::OutputFormat::Terminal {
                        check_format
                    } else {
                        format
                    };
                    match commands::rules::run_rules_check(&active_repo, target_file.as_deref()) {
                        Ok(report) => {
                            let exit_code = if report.invalid_files > 0 {
                                ExitCode::ConfigurationError
                            } else {
                                ExitCode::Success
                            };
                            match commands::rules::render_rules_check(&report, active_format) {
                                Ok(rendered) => {
                                    println!("{rendered}");
                                    exit_code
                                }
                                Err(err) => {
                                    eprintln!("Error formatting rules check report: {err}");
                                    ExitCode::ExecutionError
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error checking rule files: {err}");
                            ExitCode::ExecutionError
                        }
                    }
                }
                cli::RulesAction::Init {
                    path: init_path,
                    output,
                    force,
                    format: init_format,
                } => {
                    let active_path = if init_path != Path::new(".") {
                        init_path
                    } else {
                        path
                    };
                    let active_format = if init_format != cli::OutputFormat::Terminal {
                        init_format
                    } else {
                        format
                    };
                    match commands::rules::run_rules_init(&active_path, output.as_deref(), force) {
                        Ok(report) => {
                            match commands::rules::render_rules_init(&report, active_format) {
                                Ok(rendered) => {
                                    println!("{rendered}");
                                    ExitCode::Success
                                }
                                Err(err) => {
                                    eprintln!("Error formatting rules init report: {err}");
                                    ExitCode::ExecutionError
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error initializing rules template: {err}");
                            ExitCode::ExecutionError
                        }
                    }
                }
                cli::RulesAction::Infer {
                    path: infer_path,
                    output,
                    write,
                    format: infer_format,
                } => {
                    let active_path = if infer_path != Path::new(".") {
                        infer_path
                    } else {
                        path
                    };
                    let active_format = if infer_format != cli::OutputFormat::Terminal {
                        infer_format
                    } else {
                        format
                    };
                    match commands::rules::run_rules_infer(&active_path, output.as_deref(), write) {
                        Ok(report) => {
                            match commands::rules::render_rules_infer(&report, active_format) {
                                Ok(rendered) => {
                                    println!("{rendered}");
                                    ExitCode::Success
                                }
                                Err(err) => {
                                    eprintln!("Error formatting rules infer report: {err}");
                                    ExitCode::ExecutionError
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error inferring architectural rules: {err}");
                            ExitCode::ExecutionError
                        }
                    }
                }
            }
        }
        Some(Commands::Hook {
            action,
            path,
            format,
        }) => {
            let active_action = action.unwrap_or(cli::HookAction::Status {
                path: path.clone(),
                format,
            });
            match active_action {
                cli::HookAction::Install {
                    path: inst_path,
                    hook,
                    force,
                    format: inst_format,
                } => {
                    let active_path = if inst_path != Path::new(".") {
                        inst_path
                    } else {
                        path
                    };
                    let active_format = if inst_format != cli::OutputFormat::Terminal {
                        inst_format
                    } else {
                        format
                    };
                    match commands::hook::run_hook_install(&active_path, &hook, force) {
                        Ok(res) => match commands::hook::render_hook_install(&res, active_format) {
                            Ok(rendered) => {
                                println!("{rendered}");
                                ExitCode::Success
                            }
                            Err(err) => {
                                eprintln!("Error formatting hook install report: {err}");
                                ExitCode::ExecutionError
                            }
                        },
                        Err(err) => {
                            eprintln!("Error installing git hooks: {err}");
                            ExitCode::ExecutionError
                        }
                    }
                }
                cli::HookAction::Run {
                    hook,
                    path: run_path,
                    fail_on,
                    format: run_format,
                } => {
                    let active_path = if run_path != Path::new(".") {
                        run_path
                    } else {
                        path
                    };
                    let active_format = if run_format != cli::OutputFormat::Terminal {
                        run_format
                    } else {
                        format
                    };
                    match commands::hook::run_hook_run(&active_path, &hook, fail_on.as_deref()) {
                        Ok((report, exit_code)) => {
                            match commands::hook::render_hook_run(&report, active_format) {
                                Ok(rendered) => {
                                    println!("{rendered}");
                                    exit_code
                                }
                                Err(err) => {
                                    eprintln!("Error formatting hook run report: {err}");
                                    ExitCode::ExecutionError
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error executing git hook: {err}");
                            ExitCode::ExecutionError
                        }
                    }
                }
                cli::HookAction::Uninstall {
                    path: uninst_path,
                    hook,
                    format: uninst_format,
                } => {
                    let active_path = if uninst_path != Path::new(".") {
                        uninst_path
                    } else {
                        path
                    };
                    let active_format = if uninst_format != cli::OutputFormat::Terminal {
                        uninst_format
                    } else {
                        format
                    };
                    match commands::hook::run_hook_uninstall(&active_path, &hook) {
                        Ok(res) => match commands::hook::render_hook_uninstall(&res, active_format)
                        {
                            Ok(rendered) => {
                                println!("{rendered}");
                                ExitCode::Success
                            }
                            Err(err) => {
                                eprintln!("Error formatting hook uninstall report: {err}");
                                ExitCode::ExecutionError
                            }
                        },
                        Err(err) => {
                            eprintln!("Error uninstalling git hooks: {err}");
                            ExitCode::ExecutionError
                        }
                    }
                }
                cli::HookAction::Status {
                    path: status_path,
                    format: status_format,
                } => {
                    let active_path = if status_path != Path::new(".") {
                        status_path
                    } else {
                        path
                    };
                    let active_format = if status_format != cli::OutputFormat::Terminal {
                        status_format
                    } else {
                        format
                    };
                    match commands::hook::run_hook_status(&active_path) {
                        Ok(report) => {
                            match commands::hook::render_hook_status(&report, active_format) {
                                Ok(rendered) => {
                                    println!("{rendered}");
                                    ExitCode::Success
                                }
                                Err(err) => {
                                    eprintln!("Error formatting hook status report: {err}");
                                    ExitCode::ExecutionError
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error inspecting git hooks status: {err}");
                            ExitCode::ExecutionError
                        }
                    }
                }
            }
        }
        Some(Commands::Doctor {
            path,
            format,
            interactive,
            codeframe,
        }) => {
            if interactive {
                return match tui::run_tui(&path, tui::TuiTab::Dag, false) {
                    Ok(rendered) => {
                        println!("{rendered}");
                        ExitCode::Success
                    }
                    Err(err) => {
                        eprintln!("Error running interactive UI: {err}");
                        ExitCode::ExecutionError
                    }
                };
            }
            match commands::doctor::run_doctor(&path) {
                Ok(report) => match commands::doctor::render_doctor_report(&report, format) {
                    Ok(mut rendered) => {
                        if codeframe && format == OutputFormat::Terminal {
                            let opts = commands::scan::ScanOptions {
                                repo_root: path.clone(),
                                incremental: true,
                                format,
                            };
                            if let Ok((repo, _)) = commands::scan::execute_scan(&opts) {
                                rendered.push('\n');
                                rendered.push_str(&formatters::codeframe::render_codeframe_report(
                                    &path,
                                    &repo,
                                    &Theme::default(),
                                ));
                            }
                        }
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
            }
        }
        Some(Commands::Scan {
            path,
            format,
            full,
            incremental,
            fail_on,
            output,
            interactive,
            codeframe,
        }) => {
            if interactive {
                return match tui::run_tui(&path, tui::TuiTab::Findings, false) {
                    Ok(rendered) => {
                        println!("{rendered}");
                        ExitCode::Success
                    }
                    Err(err) => {
                        eprintln!("Error running interactive UI: {err}");
                        ExitCode::ExecutionError
                    }
                };
            }
            let opts = commands::scan::ScanOptions {
                repo_root: path.clone(),
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
                    let rendered_res = if codeframe && format == OutputFormat::Terminal {
                        render_repository_report(&repo, format).map(|mut base| {
                            base.push('\n');
                            base.push_str(&formatters::codeframe::render_codeframe_report(
                                &path,
                                &repo,
                                &Theme::default(),
                            ));
                            base
                        })
                    } else {
                        render_repository_report(&repo, format)
                    };
                    match rendered_res {
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
            interactive,
            impacted_tests,
        }) => {
            if interactive {
                return match tui::run_tui(&path, tui::TuiTab::Blast, false) {
                    Ok(rendered) => {
                        println!("{rendered}");
                        ExitCode::Success
                    }
                    Err(err) => {
                        eprintln!("Error running interactive UI: {err}");
                        ExitCode::ExecutionError
                    }
                };
            }
            match commands::review::execute_review(&path, None, fast || changed) {
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
                        Ok(mut rendered) => {
                            if impacted_tests {
                                if let Ok(tests_rep) =
                                    commands::review::execute_review_impacted_tests(&path, None)
                                {
                                    rendered.push_str(
                                        &commands::review::render_impacted_tests_appendix(
                                            &tests_rep, format,
                                        ),
                                    );
                                }
                            }
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
            }
        }
        Some(Commands::Ui { path, tab, once }) => match tui::run_tui(&path, tab, once) {
            Ok(rendered) => {
                println!("{rendered}");
                ExitCode::Success
            }
            Err(err) => {
                eprintln!("Error running TUI: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::Watch {
            path,
            debounce_ms,
            once,
            format,
        }) => match commands::watch::run_watch(&path, debounce_ms, once, format) {
            Ok(rendered) => {
                println!("{rendered}");
                ExitCode::Success
            }
            Err(err) => {
                eprintln!("Error running watch radar: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::Fix {
            path,
            dry_run,
            apply,
            rule,
            rollback,
            list_tx,
            format,
        }) => {
            let _ = dry_run;
            let rollback_ref = rollback.as_ref().map(|opt| opt.as_deref());
            match commands::fix::execute_fix(&path, apply, rule.as_deref(), rollback_ref, list_tx) {
                Ok(report) => match commands::fix::render_fix_report(&report, format) {
                    Ok(rendered) => {
                        println!("{rendered}");
                        ExitCode::Success
                    }
                    Err(err) => {
                        eprintln!("Error formatting fix report: {err}");
                        ExitCode::ExecutionError
                    }
                },
                Err(err) => {
                    eprintln!("Error running fix: {err}");
                    ExitCode::ExecutionError
                }
            }
        }
        Some(Commands::Simulate {
            action,
            plan,
            path,
            format,
        }) => {
            let mv_pair = action
                .as_ref()
                .map(|SimulateAction::Mv { from, to }| (from.as_str(), to.as_str()));
            match commands::simulate::execute_simulate(&path, mv_pair, plan.as_deref()) {
                Ok(report) => match commands::simulate::render_simulation_report(&report, format) {
                    Ok(rendered) => {
                        println!("{rendered}");
                        if report.is_safe_to_apply {
                            ExitCode::Success
                        } else {
                            ExitCode::FindingsAboveThreshold
                        }
                    }
                    Err(err) => {
                        eprintln!("Error formatting simulation report: {err}");
                        ExitCode::ExecutionError
                    }
                },
                Err(err) => {
                    eprintln!("Error running refactoring simulation: {err}");
                    ExitCode::ExecutionError
                }
            }
        }
        Some(Commands::Trend {
            path,
            commits,
            format,
        }) => match commands::trend::execute_trend(&path, commits) {
            Ok(report) => match commands::trend::render_trend_report(&report, format) {
                Ok(rendered) => {
                    println!("{rendered}");
                    ExitCode::Success
                }
                Err(err) => {
                    eprintln!("Error formatting trend report: {err}");
                    ExitCode::ExecutionError
                }
            },
            Err(err) => {
                eprintln!("Error running trend analysis: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::BlameArch {
            target,
            path,
            format,
        }) => match commands::trend::execute_blame_arch(&path, target.as_deref()) {
            Ok(entries) => match commands::trend::render_blame_arch_report(&entries, format) {
                Ok(rendered) => {
                    println!("{rendered}");
                    ExitCode::Success
                }
                Err(err) => {
                    eprintln!("Error formatting architectural blame report: {err}");
                    ExitCode::ExecutionError
                }
            },
            Err(err) => {
                eprintln!("Error running blame-arch: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::AiReview {
            path,
            base: _base,
            head: _head,
            fast,
            fail_on,
            provider,
            model,
            dry_run,
            no_cache,
            format,
        }) => {
            let opts = commands::ai_review::AiReviewOptions {
                fast,
                base: _base,
                head: _head,
                fail_on,
                provider,
                model,
                dry_run,
                no_cache,
            };
            match commands::ai_review::execute_ai_review(&path, None, &opts) {
                Ok((report, exit_code)) => {
                    match commands::ai_review::render_ai_review_report(&report, format) {
                        Ok(rendered) => {
                            println!("{rendered}");
                            exit_code
                        }
                        Err(err) => {
                            eprintln!("Error formatting ai-review report: {err}");
                            ExitCode::ExecutionError
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error running ai-review: {err}");
                    ExitCode::ExecutionError
                }
            }
        }
        Some(Commands::Diff {
            path,
            remote,
            format,
        }) => {
            if remote {
                commands::diff::run_diff_remote(&path, format)
            } else {
                match commands::diff::execute_diff(&path, None) {
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
                }
            }
        }
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
        Some(Commands::Sbom {
            path,
            format,
            output,
        }) => match commands::sbom::run_sbom(&path, format.into()) {
            Ok(rendered) => {
                if let Some(out_path) = output {
                    if let Err(err) = fs::write(&out_path, &rendered) {
                        eprintln!("Error writing SBOM to '{}': {err}", out_path.display());
                        return ExitCode::ExecutionError;
                    }
                    println!("Wrote SBOM to {}", out_path.display());
                } else {
                    println!("{rendered}");
                }
                ExitCode::Success
            }
            Err(err) => {
                eprintln!("Error generating SBOM: {err}");
                ExitCode::ExecutionError
            }
        },
        Some(Commands::Security {
            path,
            format,
            audit,
            offline,
            fail_on,
        }) => {
            match commands::security::run_security(
                &path,
                format,
                audit,
                offline,
                fail_on.as_deref(),
            ) {
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
            ai,
            provider,
            model,
            dry_run,
            no_cache,
        }) => {
            let ai_opts = ai::AiExplainOptions {
                ai,
                provider,
                model,
                dry_run,
                no_cache,
            };
            match commands::explain::execute_explain_with_ai(&path, target.as_deref(), &ai_opts) {
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
            }
        }
        Some(Commands::Report {
            path,
            output,
            hotspots,
            no_git,
            save_history,
        }) => match commands::report::run_report(
            &path,
            output.as_deref(),
            hotspots,
            no_git,
            save_history,
        ) {
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
        Some(Commands::Serve {
            path,
            port,
            allow_origin,
            static_dir,
        }) => {
            let opts = serve::ServeOptions {
                repo_root: path,
                port,
                allow_origin,
                static_dir,
            };
            match serve::run_serve(&opts) {
                Ok(()) => ExitCode::Success,
                Err(err) => {
                    eprintln!("Error running server: {err}");
                    ExitCode::ExecutionError
                }
            }
        }
        Some(Commands::Ai { action }) => match action {
            cli::AiAction::Status { path, ping, format } => {
                match commands::ai_admin::run_ai_status(&path, ping) {
                    Ok(report) => {
                        match commands::ai_admin::render_ai_status_report(&report, format) {
                            Ok(rendered) => {
                                println!("{rendered}");
                                ExitCode::Success
                            }
                            Err(err) => {
                                eprintln!("Error formatting AI status report: {err}");
                                ExitCode::ExecutionError
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("Error checking AI status: {err}");
                        ExitCode::ExecutionError
                    }
                }
            }
            cli::AiAction::Usage { path, format } => {
                match commands::ai_admin::run_ai_usage(&path) {
                    Ok(report) => {
                        match commands::ai_admin::render_ai_usage_report(&report, format) {
                            Ok(rendered) => {
                                println!("{rendered}");
                                ExitCode::Success
                            }
                            Err(err) => {
                                eprintln!("Error formatting AI usage report: {err}");
                                ExitCode::ExecutionError
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("Error retrieving AI usage: {err}");
                        ExitCode::ExecutionError
                    }
                }
            }
        },
        Some(Commands::Login {
            server,
            token,
            username,
            no_validate,
        }) => commands::auth::run_auth_login(
            &server,
            token.as_deref(),
            username.as_deref(),
            no_validate,
        ),
        Some(Commands::Logout { server }) => commands::auth::run_auth_logout(server.as_deref()),
        Some(Commands::Auth { action }) => match action {
            cli::AuthAction::Login {
                server,
                token,
                username,
                no_validate,
            } => commands::auth::run_auth_login(
                &server,
                token.as_deref(),
                username.as_deref(),
                no_validate,
            ),
            cli::AuthAction::Logout { server } => {
                commands::auth::run_auth_logout(server.as_deref())
            }
            cli::AuthAction::Status { server, format } => {
                commands::auth::run_auth_status(server.as_deref(), format)
            }
        },
        Some(Commands::Push {
            path,
            server,
            token,
            branch,
            format,
        }) => commands::sync::run_sync_push(
            &path,
            server.as_deref(),
            token.as_deref(),
            branch.as_deref(),
            format,
        ),
        Some(Commands::Pull {
            path,
            server,
            token,
            branch,
            format,
        }) => commands::sync::run_sync_pull(
            &path,
            server.as_deref(),
            token.as_deref(),
            branch.as_deref(),
            format,
        ),
        Some(Commands::Sync {
            path,
            server,
            token,
            branch,
            fail_on,
            format,
        }) => commands::sync::run_sync_pipeline(
            &path,
            server.as_deref(),
            token.as_deref(),
            branch.as_deref(),
            fail_on.as_deref(),
            format,
        ),
        Some(Commands::Policy {
            action,
            policy,
            path,
            fail_on,
            format,
        }) => {
            let (target_policy, target_path, target_fail_on, target_format) = match action {
                Some(cli::PolicyAction::Check {
                    policy: check_policy,
                    path: check_path,
                    fail_on: check_fail_on,
                    format: check_format,
                }) => {
                    let p = check_policy.or(policy);
                    let dir = if check_path != Path::new(".") {
                        check_path
                    } else {
                        path
                    };
                    let fo = check_fail_on.or(fail_on);
                    let fmt = if check_format != cli::OutputFormat::Terminal {
                        check_format
                    } else {
                        format
                    };
                    (p, dir, fo, fmt)
                }
                None => (policy, path, fail_on, format),
            };

            let fail_on_sev = target_fail_on.as_deref().and_then(parse_severity_threshold);
            commands::policy::run_policy_check(
                &target_path,
                target_policy.as_deref(),
                target_format,
                fail_on_sev,
            )
        }
        Some(Commands::Audit {
            standard,
            path,
            format,
            output,
        }) => commands::audit::run_audit(&path, standard.into(), format, output.as_deref()),
    }
}
