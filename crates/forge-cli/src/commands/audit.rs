//! `eforge audit` command — automated compliance framework auditing (SOC 2, OWASP Top 10, ISO 27001).

use std::fs;
use std::path::Path;

use forge_core::error::ForgeError;
use forge_security::compliance::{
    evaluate_compliance, render_compliance_markdown, render_compliance_terminal, ComplianceReport,
    ComplianceStandard,
};

use crate::cli::OutputFormat;
use crate::commands::scan::{execute_scan, ScanOptions};
use crate::exit_code::ExitCode;

/// Evaluates repository findings against an enterprise compliance standard.
pub fn execute_audit(
    repo_root: &Path,
    standard: ComplianceStandard,
) -> Result<ComplianceReport, ForgeError> {
    let scan_opts = ScanOptions {
        repo_root: repo_root.to_path_buf(),
        incremental: false,
        format: OutputFormat::Json,
    };
    let (repo, _) = execute_scan(&scan_opts)?;

    let report = evaluate_compliance(standard, &repo.findings, &repo.dependencies);
    Ok(report)
}

/// Renders a [`ComplianceReport`] into the specified [`OutputFormat`].
pub fn render_audit_report(
    report: &ComplianceReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json => {
            serde_json::to_string_pretty(report).map_err(|err| ForgeError::Serialization {
                message: format!("Failed to serialize ComplianceReport to JSON: {err}"),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => Ok(render_compliance_markdown(report)),
        OutputFormat::Terminal | OutputFormat::Sarif => Ok(render_compliance_terminal(report)),
    }
}

/// Primary orchestrator for the `eforge audit` CLI invocation.
pub fn run_audit(
    repo_root: &Path,
    standard: ComplianceStandard,
    format: OutputFormat,
    output: Option<&Path>,
) -> ExitCode {
    match execute_audit(repo_root, standard) {
        Ok(report) => {
            let rendered = match render_audit_report(&report, format) {
                Ok(r) => r,
                Err(err) => {
                    eprintln!("Error formatting compliance report: {err}");
                    return ExitCode::ExecutionError;
                }
            };

            if let Some(out_path) = output {
                let target = if out_path.is_relative() {
                    repo_root.join(out_path)
                } else {
                    out_path.to_path_buf()
                };

                if let Some(parent) = target.parent() {
                    let _ = fs::create_dir_all(parent);
                }

                if let Err(source) = fs::write(&target, &rendered) {
                    eprintln!(
                        "Error writing audit report to {}: {source}",
                        target.display()
                    );
                    return ExitCode::ExecutionError;
                }
                println!("Audit report written to {}", target.display());
            } else {
                print!("{rendered}");
            }

            if report.overall_passed {
                ExitCode::Success
            } else {
                ExitCode::SecurityPolicyFailure
            }
        }
        Err(err) => {
            eprintln!("Error executing compliance audit: {err}");
            ExitCode::ExecutionError
        }
    }
}
