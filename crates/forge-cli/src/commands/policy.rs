//! `eforge policy check` command — enterprise Policy-as-Code evaluation and exit code enforcement.

use std::path::Path;

use forge_core::config::load_config;
use forge_core::error::ForgeError;
use forge_core::ir::Severity;
use forge_core::policy::{
    load_policy, PolicyDocument, PolicyImpactSummary, PolicyReport, PolicyVerdict,
};
use forge_core::scoring::calculate_health_score;
use forge_git::diff::collect_working_tree_diff;
use forge_git::diff::parse_unified_diff;
use forge_git::impact::compute_change_impact;
use forge_rules::policy_engine::evaluate_policy;

use crate::cli::OutputFormat;
use crate::commands::scan::{execute_scan, ScanOptions};
use crate::exit_code::ExitCode;
use crate::theme::Theme;

/// Evaluates repository findings, scores, and change impact against organizational policy.
pub fn execute_policy_check(
    repo_root: &Path,
    policy_path: Option<&Path>,
) -> Result<PolicyReport, ForgeError> {
    let config = load_config(repo_root)?;

    // 1. Resolve policy document: explicit CLI flag takes precedence, fallback to config
    let policy: PolicyDocument = if let Some(path) = policy_path {
        let target_path = if path.is_relative() {
            repo_root.join(path)
        } else {
            path.to_path_buf()
        };
        load_policy(&target_path)?
    } else if let Some(ref pol) = config.policy {
        pol.clone()
    } else {
        return Err(ForgeError::Config {
            message: "No Policy-as-Code document found. Specify --policy <path> or declare a 'policy:' block in eforge.config.yaml".to_string(),
        });
    };

    // 2. Execute local scan to collect repository IR, findings, dependencies, and health
    let scan_opts = ScanOptions {
        repo_root: repo_root.to_path_buf(),
        incremental: false,
        format: OutputFormat::Json,
    };
    let (repo, _) = execute_scan(&scan_opts)?;
    let health = match repo.health {
        Some(ref h) => h.clone(),
        None => calculate_health_score(&repo),
    };

    // 3. Inspect git diff blast radius if git repository is present
    let impact_summary: Option<PolicyImpactSummary> =
        if config.git_enabled && repo_root.join(".git").exists() {
            if let Ok(diff_text) = collect_working_tree_diff(repo_root) {
                if let Ok(changes) = parse_unified_diff(&diff_text) {
                    let test_files = repo
                        .files
                        .iter()
                        .filter(|f| f.is_test)
                        .map(|f| f.relative_path.clone())
                        .collect();
                    let impact_report = compute_change_impact(&changes, &repo.modules, &test_files);
                    Some(PolicyImpactSummary::from(&impact_report))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

    // 4. Pure policy evaluation pass
    let report = evaluate_policy(
        &policy,
        &repo.findings,
        &health,
        impact_summary.as_ref(),
        &repo.dependencies,
        &config.rule_packs,
    );

    Ok(report)
}

/// Computes the deterministic [`ExitCode`] for a policy check run.
///
/// - `0` (Success): Policy passed or warning below failure threshold.
/// - `1` (FindingsAboveThreshold): Policy warning with fail-on threshold satisfied.
/// - `4` (SecurityPolicyFailure): Policy violation.
pub fn compute_policy_exit_code(report: &PolicyReport, fail_on: Option<Severity>) -> ExitCode {
    match report.verdict {
        PolicyVerdict::Violation => ExitCode::SecurityPolicyFailure,
        PolicyVerdict::Warning => {
            if let Some(threshold) = fail_on {
                if threshold <= Severity::Medium {
                    ExitCode::FindingsAboveThreshold
                } else {
                    ExitCode::Success
                }
            } else {
                ExitCode::Success
            }
        }
        PolicyVerdict::Passed => ExitCode::Success,
    }
}

/// Renders a [`PolicyReport`] into the requested [`OutputFormat`].
pub fn render_policy_report(
    report: &PolicyReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json => {
            serde_json::to_string_pretty(report).map_err(|err| ForgeError::Serialization {
                message: format!("Failed to serialize PolicyReport to JSON: {err}"),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => Ok(render_markdown_policy(report)),
        OutputFormat::Terminal | OutputFormat::Sarif => Ok(render_terminal_policy(report)),
    }
}

fn render_terminal_policy(report: &PolicyReport) -> String {
    let theme = Theme::default();
    let mut out = String::new();

    out.push_str("============================================================\n");
    out.push_str(&format!(
        " {}\n",
        theme.bold(&format!(
            "ELCODE Forge Policy Gate — {}",
            report.policy_name
        ))
    ));
    out.push_str("============================================================\n");

    let verdict_str = match report.verdict {
        PolicyVerdict::Passed => theme.green(&theme.bold("PASSED")),
        PolicyVerdict::Warning => theme.yellow(&theme.bold("WARNING")),
        PolicyVerdict::Violation => theme.red(&theme.bold("VIOLATION")),
    };
    out.push_str(&format!("Verdict:           {}\n", verdict_str));
    out.push_str(&format!(
        "Summary:           {} passed, {} warning(s), {} violation(s)\n\n",
        report.passed_checks.len(),
        report.warnings.len(),
        report.violations.len()
    ));

    if !report.violations.is_empty() {
        let bullet = if theme.unicode_enabled { "✗" } else { "X" };
        out.push_str(&format!("{}\n", theme.red(&theme.bold("Violations:"))));
        for v in &report.violations {
            out.push_str(&format!("  {} {}\n", theme.red(bullet), v));
        }
        out.push('\n');
    }

    if !report.warnings.is_empty() {
        let bullet = if theme.unicode_enabled { "⚠" } else { "!" };
        out.push_str(&format!("{}\n", theme.yellow(&theme.bold("Warnings:"))));
        for w in &report.warnings {
            out.push_str(&format!("  {} {}\n", theme.yellow(bullet), w));
        }
        out.push('\n');
    }

    if !report.passed_checks.is_empty() {
        let bullet = if theme.unicode_enabled { "✓" } else { "+" };
        out.push_str(&format!("{}\n", theme.green(&theme.bold("Passed Checks:"))));
        for p in &report.passed_checks {
            out.push_str(&format!("  {} {}\n", theme.green(bullet), p));
        }
        out.push('\n');
    }

    if !report.active_waivers.is_empty() {
        out.push_str(&format!("{}\n", theme.bold("Active Waivers:")));
        for w in &report.active_waivers {
            let path = w.path_pattern.as_deref().unwrap_or("*");
            let exp = w.expires_at.as_deref().unwrap_or("permanent");
            out.push_str(&format!(
                "  - Rule: {} | Scope: {} | Expires: {}\n    Justification: {}\n",
                theme.cyan(&w.rule_id),
                path,
                exp,
                w.justification
            ));
        }
        out.push('\n');
    }

    out
}

fn render_markdown_policy(report: &PolicyReport) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "# Policy-as-Code Gate Report: {}\n\n",
        report.policy_name
    ));

    let verdict_badge = match report.verdict {
        PolicyVerdict::Passed => "**PASSED** :white_check_mark:",
        PolicyVerdict::Warning => "**WARNING** :warning:",
        PolicyVerdict::Violation => "**VIOLATION** :x:",
    };

    out.push_str(&format!("- **Verdict:** {}\n", verdict_badge));
    out.push_str(&format!(
        "- **Checks Passed:** {}\n- **Warnings:** {}\n- **Violations:** {}\n\n",
        report.passed_checks.len(),
        report.warnings.len(),
        report.violations.len()
    ));

    if !report.violations.is_empty() {
        out.push_str("### Policy Violations\n\n");
        for v in &report.violations {
            out.push_str(&format!("- :x: {}\n", v));
        }
        out.push('\n');
    }

    if !report.warnings.is_empty() {
        out.push_str("### Policy Warnings\n\n");
        for w in &report.warnings {
            out.push_str(&format!("- :warning: {}\n", w));
        }
        out.push('\n');
    }

    if !report.passed_checks.is_empty() {
        out.push_str("### Passed Checks\n\n");
        for p in &report.passed_checks {
            out.push_str(&format!("- :white_check_mark: {}\n", p));
        }
        out.push('\n');
    }

    if !report.active_waivers.is_empty() {
        out.push_str("### Active Waivers\n\n");
        out.push_str("| Rule ID | Path Scope | Expiration | Justification |\n");
        out.push_str("| :--- | :--- | :--- | :--- |\n");
        for w in &report.active_waivers {
            let path = w.path_pattern.as_deref().unwrap_or("`*`");
            let exp = w.expires_at.as_deref().unwrap_or("permanent");
            out.push_str(&format!(
                "| `{}` | `{}` | {} | {} |\n",
                w.rule_id, path, exp, w.justification
            ));
        }
        out.push('\n');
    }

    out
}

/// Primary orchestrator for the `eforge policy check` CLI invocation.
pub fn run_policy_check(
    repo_root: &Path,
    policy_path: Option<&Path>,
    format: OutputFormat,
    fail_on: Option<Severity>,
) -> ExitCode {
    match execute_policy_check(repo_root, policy_path) {
        Ok(report) => {
            let exit_code = compute_policy_exit_code(&report, fail_on);
            match render_policy_report(&report, format) {
                Ok(output) => {
                    print!("{output}");
                    exit_code
                }
                Err(err) => {
                    eprintln!("Error rendering policy report: {err}");
                    ExitCode::ExecutionError
                }
            }
        }
        Err(ForgeError::Config { message }) => {
            eprintln!("Configuration error: {message}");
            ExitCode::ConfigurationError
        }
        Err(err) => {
            eprintln!("Error executing policy check: {err}");
            ExitCode::ExecutionError
        }
    }
}
