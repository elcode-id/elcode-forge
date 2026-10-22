//! `eforge security` command — focused secret detection, unsafe pattern analysis,
//! and polyglot OSV dependency vulnerability audit.

use std::collections::BTreeSet;
use std::path::Path;

use forge_analyzer::dep_health::analyze_polyglot_dependencies;
use forge_analyzer::scanner::scan_repository;
use forge_core::{config::load_config, Finding, ForgeError, Repository, Severity};
use forge_security::{SecurityScanner, RULE_VULNERABLE_DEPENDENCY};

use crate::cli::OutputFormat;
use crate::exit_code::{parse_severity_threshold, ExitCode};
use crate::formatters::{render_repository_report, render_sarif};

/// Runs the dedicated security & secret scanner or OSV vulnerability audit on `repo_root`.
pub fn run_security(
    repo_root: &Path,
    format: OutputFormat,
    audit: bool,
    _offline: bool,
    fail_on: Option<&str>,
) -> Result<(String, ExitCode), ForgeError> {
    let config = load_config(repo_root)?;
    let discovery = scan_repository(repo_root, &config.ignore_patterns)?;

    // 1. Scan source files for secrets and unsafe language patterns
    let mut findings = SecurityScanner::scan_repository_files(repo_root, &discovery.files)?;

    // 2. If audit mode is enabled, analyze dependencies and match against OSV advisories
    let mut dependencies = Vec::new();
    if audit {
        let external_imports = BTreeSet::new();
        let dep_report =
            analyze_polyglot_dependencies(repo_root, &discovery.files, &external_imports)?;
        dependencies = dep_report.dependencies;
        let vuln_findings = SecurityScanner::scan_dependencies(repo_root, &dependencies);
        findings.extend(vuln_findings);
    }

    // Deterministic sorting: severity descending, then finding ID ascending
    findings.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
    findings.dedup_by(|a, b| a.id == b.id && a.rule_id == b.rule_id && a.location == b.location);

    let effective_threshold = fail_on
        .and_then(parse_severity_threshold)
        .or(config.fail_on_severity);
    let exit_code = ExitCode::evaluate_findings(&findings, effective_threshold);

    let repo = Repository {
        root_path: repo_root.to_string_lossy().replace('\\', "/"),
        name: config.project_name.unwrap_or_else(|| {
            repo_root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repository")
                .to_string()
        }),
        detected_languages: discovery.languages,
        detected_frameworks: discovery.frameworks,
        package_managers: discovery.package_managers,
        files: discovery.files,
        modules: Vec::new(),
        dependencies,
        findings,
        health: None,
    };

    let rendered = match format {
        OutputFormat::Json => serde_json::to_string_pretty(&repo.findings).map_err(|err| {
            ForgeError::Serialization {
                message: err.to_string(),
            }
        })?,
        OutputFormat::Sarif => render_sarif(&repo)?,
        OutputFormat::Terminal if audit => render_terminal_security_audit(&repo),
        _ => render_repository_report(&repo, format)?,
    };

    Ok((rendered, exit_code))
}

/// Formats a dedicated terminal summary for `eforge security --audit`.
fn render_terminal_security_audit(repo: &Repository) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    let vuln_findings: Vec<&Finding> = repo
        .findings
        .iter()
        .filter(|f| f.rule_id == RULE_VULNERABLE_DEPENDENCY)
        .collect();

    let other_findings: Vec<&Finding> = repo
        .findings
        .iter()
        .filter(|f| f.rule_id != RULE_VULNERABLE_DEPENDENCY)
        .collect();

    let mut crit = 0;
    let mut high = 0;
    let mut med = 0;
    let mut low = 0;

    for vf in &vuln_findings {
        match vf.severity {
            Severity::Critical => crit += 1,
            Severity::High => high += 1,
            Severity::Medium => med += 1,
            Severity::Low | Severity::Info => low += 1,
        }
    }

    let _ = writeln!(out, "Dependency Vulnerability Audit — {}", repo.name);
    let _ = writeln!(out, "==================================================");
    let _ = writeln!(out, "Summary:");
    let _ = writeln!(out, "  Critical: {crit}");
    let _ = writeln!(out, "  High:     {high}");
    let _ = writeln!(out, "  Medium:   {med}");
    let _ = writeln!(out, "  Low:      {low}");
    let _ = writeln!(out, "  Total:    {}", vuln_findings.len());
    let _ = writeln!(out);

    if !vuln_findings.is_empty() {
        let _ = writeln!(out, "Vulnerabilities:");
        for vf in &vuln_findings {
            let sev_str = match vf.severity {
                Severity::Critical => "[CRITICAL]",
                Severity::High => "[HIGH]    ",
                Severity::Medium => "[MEDIUM]  ",
                Severity::Low => "[LOW]     ",
                Severity::Info => "[INFO]    ",
            };
            let loc_str = vf
                .location
                .as_ref()
                .map(|l| l.file_path.as_str())
                .unwrap_or("manifest");
            let _ = writeln!(out, "  {sev_str} {}", vf.title);
            let _ = writeln!(out, "             Location: {loc_str}");
            let _ = writeln!(out, "             Summary:  {}", vf.description);
            let _ = writeln!(out, "             Remedy:   {}", vf.recommendation);
            let _ = writeln!(out);
        }
    }

    if !other_findings.is_empty() {
        let _ = writeln!(
            out,
            "Secrets & Code Pattern Findings ({}):",
            other_findings.len()
        );
        for of in &other_findings {
            let loc_str = of
                .location
                .as_ref()
                .map(|l| {
                    if let Some(line) = l.start_line {
                        format!("{}:{}", l.file_path, line)
                    } else {
                        l.file_path.clone()
                    }
                })
                .unwrap_or_else(|| "unknown".to_string());
            let _ = writeln!(out, "  [{:?}] {} — {}", of.severity, loc_str, of.title);
        }
        let _ = writeln!(out);
    }

    if vuln_findings.is_empty() && other_findings.is_empty() {
        let _ = writeln!(
            out,
            "No security vulnerabilities or secret findings detected."
        );
    }

    out
}
