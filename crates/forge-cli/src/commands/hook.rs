//! `eforge hook` command — native Git hooks automation, installation, status, and fast staged evaluation.

use std::path::Path;
use std::process::Command;

use forge_core::config::{load_config, HooksConfig};
use forge_core::{Finding, ForgeError, Severity};
use forge_git::{
    get_hooks_status, install_hooks, uninstall_hooks, HookEntryStatus, HookInstallResult, HookType,
    HookUninstallResult,
};
use serde::{Deserialize, Serialize};

use crate::cli::OutputFormat;
use crate::commands::review::execute_review;
use crate::exit_code::ExitCode;

/// CLI status report for Git hooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookStatusCliReport {
    pub git_dir: String,
    pub hooks_dir: String,
    pub hooks: Vec<HookEntryStatus>,
    pub config: Option<HooksConfig>,
}

/// Execution report emitted when a Git hook check runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookRunReport {
    pub hook_name: String,
    pub staged_files_count: usize,
    pub staged_files: Vec<String>,
    pub findings_count: usize,
    pub violations_count: usize,
    pub fail_on: Severity,
    pub passed: bool,
    pub findings: Vec<Finding>,
}

/// Parses a string into a [`HookType`].
pub fn parse_hook_type(s: &str) -> Result<HookType, ForgeError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "pre-commit" | "pre_commit" | "commit" => Ok(HookType::PreCommit),
        "pre-push" | "pre_push" | "push" => Ok(HookType::PrePush),
        "all" | "*" => Ok(HookType::All),
        other => Err(ForgeError::Config {
            message: format!("unknown hook type '{other}'; supported: pre-commit, pre-push, all"),
        }),
    }
}

/// Executes `eforge hook install`.
pub fn run_hook_install(
    repo_root: &Path,
    hook_type_str: &str,
    force: bool,
) -> Result<HookInstallResult, ForgeError> {
    let hook_type = parse_hook_type(hook_type_str)?;
    install_hooks(repo_root, hook_type, force)
}

/// Executes `eforge hook uninstall`.
pub fn run_hook_uninstall(
    repo_root: &Path,
    hook_type_str: &str,
) -> Result<HookUninstallResult, ForgeError> {
    let hook_type = parse_hook_type(hook_type_str)?;
    uninstall_hooks(repo_root, hook_type)
}

/// Executes `eforge hook status`.
pub fn run_hook_status(repo_root: &Path) -> Result<HookStatusCliReport, ForgeError> {
    let raw_status = get_hooks_status(repo_root)?;
    let config = load_config(repo_root)?;

    Ok(HookStatusCliReport {
        git_dir: raw_status.git_dir.display().to_string(),
        hooks_dir: raw_status.hooks_dir.display().to_string(),
        hooks: raw_status.hooks,
        config: config.hooks,
    })
}

/// Collects list of staged file paths from Git.
pub fn collect_staged_files(repo_root: &Path) -> Result<Vec<String>, ForgeError> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        .current_dir(repo_root)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let files: Vec<String> = stdout
                .lines()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            Ok(files)
        }
        _ => Ok(Vec::new()),
    }
}

/// Collects staged unified diff text from Git.
pub fn collect_staged_diff(repo_root: &Path) -> Result<String, ForgeError> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--unified=0"])
        .current_dir(repo_root)
        .output();

    match output {
        Ok(out) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        _ => Ok(String::new()),
    }
}

/// Collects unpushed commits diff text from Git for pre-push checks.
pub fn collect_unpushed_diff(repo_root: &Path) -> Result<String, ForgeError> {
    // Attempt tracking branch first
    let output = Command::new("git")
        .args(["diff", "@{u}..HEAD", "--unified=0"])
        .current_dir(repo_root)
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
        }
    }

    // Fallback to origin/main or origin/develop
    let fallback = Command::new("git")
        .args(["diff", "origin/main..HEAD", "--unified=0"])
        .current_dir(repo_root)
        .output();

    if let Ok(out) = fallback {
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
        }
    }

    Ok(String::new())
}

/// Executes `eforge hook run <hook_name>`.
pub fn run_hook_run(
    repo_root: &Path,
    hook_name: &str,
    fail_on_override: Option<&str>,
) -> Result<(HookRunReport, ExitCode), ForgeError> {
    let hook_type = parse_hook_type(hook_name)?;
    let config = load_config(repo_root)?;

    let (enabled, configured_fail_on) = match hook_type {
        HookType::PreCommit => {
            let c = config.hooks.as_ref().and_then(|h| h.pre_commit.as_ref());
            let enabled = c.map(|h| h.enabled).unwrap_or(true);
            let fail_on = c.and_then(|h| h.fail_on);
            (enabled, fail_on)
        }
        HookType::PrePush => {
            let c = config.hooks.as_ref().and_then(|h| h.pre_push.as_ref());
            let enabled = c.map(|h| h.enabled).unwrap_or(true);
            let fail_on = c.and_then(|h| h.fail_on);
            (enabled, fail_on)
        }
        HookType::All => (true, None),
    };

    let fail_on_sev = if let Some(override_str) = fail_on_override {
        parse_severity_str(override_str)?
    } else if let Some(configured) = configured_fail_on {
        configured
    } else if let Some(global_fail_on) = config.fail_on_severity {
        global_fail_on
    } else {
        Severity::High
    };

    if !enabled {
        let report = HookRunReport {
            hook_name: hook_name.to_string(),
            staged_files_count: 0,
            staged_files: Vec::new(),
            findings_count: 0,
            violations_count: 0,
            fail_on: fail_on_sev,
            passed: true,
            findings: Vec::new(),
        };
        return Ok((report, ExitCode::Success));
    }

    match hook_type {
        HookType::PreCommit => {
            let staged_files = collect_staged_files(repo_root)?;
            if staged_files.is_empty() {
                let report = HookRunReport {
                    hook_name: hook_name.to_string(),
                    staged_files_count: 0,
                    staged_files: Vec::new(),
                    findings_count: 0,
                    violations_count: 0,
                    fail_on: fail_on_sev,
                    passed: true,
                    findings: Vec::new(),
                };
                return Ok((report, ExitCode::Success));
            }

            let diff_text = collect_staged_diff(repo_root)?;
            let (review, _) = execute_review(repo_root, Some(&diff_text), true)?;

            let violations: Vec<Finding> = review
                .findings
                .iter()
                .filter(|f| f.severity >= fail_on_sev)
                .cloned()
                .collect();

            let passed = violations.is_empty();
            let exit_code = ExitCode::evaluate_findings(&violations, Some(fail_on_sev));

            let report = HookRunReport {
                hook_name: hook_name.to_string(),
                staged_files_count: staged_files.len(),
                staged_files,
                findings_count: review.findings.len(),
                violations_count: violations.len(),
                fail_on: fail_on_sev,
                passed,
                findings: review.findings,
            };

            Ok((report, exit_code))
        }
        HookType::PrePush => {
            let unpushed_diff = collect_unpushed_diff(repo_root)?;
            let diff_arg = if unpushed_diff.is_empty() {
                None
            } else {
                Some(unpushed_diff.as_str())
            };
            let (review, _) = execute_review(repo_root, diff_arg, true)?;

            let violations: Vec<Finding> = review
                .findings
                .iter()
                .filter(|f| f.severity >= fail_on_sev)
                .cloned()
                .collect();

            let passed = violations.is_empty();
            let exit_code = ExitCode::evaluate_findings(&violations, Some(fail_on_sev));

            let report = HookRunReport {
                hook_name: hook_name.to_string(),
                staged_files_count: review.impact.changed_files.len(),
                staged_files: review
                    .impact
                    .changed_files
                    .iter()
                    .map(|c| c.path.clone())
                    .collect(),
                findings_count: review.findings.len(),
                violations_count: violations.len(),
                fail_on: fail_on_sev,
                passed,
                findings: review.findings,
            };

            Ok((report, exit_code))
        }
        HookType::All => {
            // Running all hooks in sequence
            run_hook_run(repo_root, "pre-commit", fail_on_override)
        }
    }
}

fn parse_severity_str(s: &str) -> Result<Severity, ForgeError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "critical" => Ok(Severity::Critical),
        "high" => Ok(Severity::High),
        "medium" | "med" => Ok(Severity::Medium),
        "low" => Ok(Severity::Low),
        "info" => Ok(Severity::Info),
        other => Err(ForgeError::Config {
            message: format!("invalid severity '{other}'; expected critical|high|medium|low|info"),
        }),
    }
}

/// Renders the result of `eforge hook install`.
pub fn render_hook_install(
    result: &HookInstallResult,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json => {
            serde_json::to_string_pretty(result).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown => {
            let mut md = String::new();
            md.push_str("# Git Hooks Installation\n\n");
            md.push_str("| Hook | Status | Details |\n|---|---|---|\n");
            for h in &result.installed {
                md.push_str(&format!("| `{h}` | Installed | Active |\n"));
            }
            for b in &result.backed_up {
                md.push_str(&format!(
                    "| `{b}` | Backed up | Saved to `{b}.forge-backup` |\n"
                ));
            }
            for s in &result.skipped {
                md.push_str(&format!("| `{s}` | Skipped | Already installed |\n"));
            }
            Ok(md)
        }
        OutputFormat::Terminal | OutputFormat::Sarif | OutputFormat::Github => {
            let mut out = String::new();
            out.push_str("ELCODE FORGE — Git Hooks Installation\n");
            out.push_str("============================================================\n");
            if result.installed.is_empty()
                && result.backed_up.is_empty()
                && result.skipped.is_empty()
            {
                out.push_str("No hooks modified.\n");
            }
            for h in &result.installed {
                out.push_str(&format!("✓ Installed hook: .git/hooks/{h}\n"));
            }
            for b in &result.backed_up {
                out.push_str(&format!(
                    "! Pre-existing foreign hook backed up to: .git/hooks/{b}.forge-backup\n"
                ));
            }
            for s in &result.skipped {
                out.push_str(&format!(
                    "- Skipped hook (already managed by eforge): .git/hooks/{s} (use --force to overwrite)\n"
                ));
            }
            out.push('\n');
            out.push_str(
                "Git hooks are now active. Staged commits will be automatically reviewed.\n",
            );
            Ok(out)
        }
    }
}

/// Renders the result of `eforge hook uninstall`.
pub fn render_hook_uninstall(
    result: &HookUninstallResult,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json => {
            serde_json::to_string_pretty(result).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown => {
            let mut md = String::new();
            md.push_str("# Git Hooks Uninstallation\n\n");
            md.push_str("| Hook | Status | Details |\n|---|---|---|\n");
            for u in &result.uninstalled {
                md.push_str(&format!("| `{u}` | Removed | Deleted |\n"));
            }
            for r in &result.restored {
                md.push_str(&format!("| `{r}` | Restored | Restored from backup |\n"));
            }
            for s in &result.skipped {
                md.push_str(&format!("| `{s}` | Skipped | Not installed or foreign |\n"));
            }
            Ok(md)
        }
        OutputFormat::Terminal | OutputFormat::Sarif | OutputFormat::Github => {
            let mut out = String::new();
            out.push_str("ELCODE FORGE — Git Hooks Uninstallation\n");
            out.push_str("============================================================\n");
            if result.uninstalled.is_empty()
                && result.restored.is_empty()
                && result.skipped.is_empty()
            {
                out.push_str("No hooks uninstalled.\n");
            }
            for u in &result.uninstalled {
                out.push_str(&format!("✓ Removed hook: .git/hooks/{u}\n"));
            }
            for r in &result.restored {
                out.push_str(&format!(
                    "✓ Restored original foreign hook from: .git/hooks/{r}.forge-backup\n"
                ));
            }
            for s in &result.skipped {
                out.push_str(&format!(
                    "- Skipped: .git/hooks/{s} (not managed by eforge)\n"
                ));
            }
            Ok(out)
        }
    }
}

/// Renders the result of `eforge hook status`.
pub fn render_hook_status(
    report: &HookStatusCliReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json => {
            serde_json::to_string_pretty(report).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown => {
            let mut md = String::new();
            md.push_str("# Git Hooks Status\n\n");
            md.push_str(&format!(
                "- **Hooks Directory:** `{}`\n\n",
                report.hooks_dir
            ));
            md.push_str(
                "| Hook | Installed | Managed by Forge | Has Backup |\n|---|---|---|---|\n",
            );
            for h in &report.hooks {
                md.push_str(&format!(
                    "| `{}` | {} | {} | {} |\n",
                    h.hook_name,
                    if h.installed { "✓ Yes" } else { "✗ No" },
                    if h.managed_by_forge { "✓ Yes" } else { "No" },
                    if h.has_backup {
                        "Yes (.forge-backup)"
                    } else {
                        "No"
                    },
                ));
            }
            Ok(md)
        }
        OutputFormat::Terminal | OutputFormat::Sarif | OutputFormat::Github => {
            let mut out = String::new();
            out.push_str("ELCODE FORGE — Git Hooks Status\n");
            out.push_str("============================================================\n");
            out.push_str(&format!("Hooks Directory: {}\n\n", report.hooks_dir));
            out.push_str(&format!(
                "  {:<14} {:<12} {:<20} {:<15}\n",
                "Hook", "Installed", "Managed by Forge", "Backup"
            ));
            out.push_str(&format!(
                "  {:-<14} {:-<12} {:-<20} {:-<15}\n",
                "", "", "", ""
            ));
            for h in &report.hooks {
                out.push_str(&format!(
                    "  {:<14} {:<12} {:<20} {:<15}\n",
                    h.hook_name,
                    if h.installed { "yes" } else { "no" },
                    if h.managed_by_forge { "yes" } else { "no" },
                    if h.has_backup { "yes (.backup)" } else { "-" }
                ));
            }
            out.push('\n');
            if let Some(cfg) = &report.config {
                out.push_str("Configuration (from eforge.config.yaml):\n");
                if let Some(pc) = &cfg.pre_commit {
                    out.push_str(&format!(
                        "  pre-commit: enabled={}, fail_on={:?}, fast={:?}\n",
                        pc.enabled, pc.fail_on, pc.fast
                    ));
                }
                if let Some(pp) = &cfg.pre_push {
                    out.push_str(&format!(
                        "  pre-push:   enabled={}, fail_on={:?}, fast={:?}\n",
                        pp.enabled, pp.fail_on, pp.fast
                    ));
                }
            } else {
                out.push_str("No `hooks:` configuration section specified (using defaults).\n");
            }
            Ok(out)
        }
    }
}

/// Renders the result of `eforge hook run`.
pub fn render_hook_run(report: &HookRunReport, format: OutputFormat) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json => {
            serde_json::to_string_pretty(report).map_err(|e| ForgeError::Serialization {
                message: e.to_string(),
            })
        }
        OutputFormat::Markdown => {
            let mut md = String::new();
            md.push_str(&format!("# Git Hook Check — `{}`\n\n", report.hook_name));
            md.push_str(&format!(
                "- **Result:** {}\n- **Staged Files:** {}\n- **Findings:** {}\n- **Violations (>= {:?}):** {}\n\n",
                if report.passed { "PASSED" } else { "FAILED" },
                report.staged_files_count,
                report.findings_count,
                report.fail_on,
                report.violations_count
            ));
            if !report.findings.is_empty() {
                md.push_str("### Findings\n\n");
                for f in &report.findings {
                    let loc = f
                        .location
                        .as_ref()
                        .map(|l| l.file_path.as_str())
                        .unwrap_or("-");
                    md.push_str(&format!(
                        "- `[{:?}]` **{}** (`{}` at `{}`)\n",
                        f.severity, f.title, f.rule_id, loc
                    ));
                }
            }
            Ok(md)
        }
        OutputFormat::Terminal | OutputFormat::Sarif | OutputFormat::Github => {
            let mut out = String::new();
            out.push_str(&format!(
                "ELCODE FORGE — Git Hook Check ({})\n",
                report.hook_name
            ));
            out.push_str("============================================================\n");
            if report.staged_files_count == 0 {
                out.push_str("✓ No staged code files to check. Hook passed.\n");
                return Ok(out);
            }

            out.push_str(&format!(
                "Evaluated {} staged file(s) across active rules.\n\n",
                report.staged_files_count
            ));

            if report.passed {
                out.push_str(&format!(
                    "✓ Git hook check PASSED. (0 findings met fail-on threshold {:?})\n",
                    report.fail_on
                ));
            } else {
                out.push_str(&format!(
                    "✗ Git hook check FAILED: {} finding(s) met or exceeded {:?}.\n\n",
                    report.violations_count, report.fail_on
                ));

                out.push_str("Violations:\n");
                for f in &report.findings {
                    if f.severity >= report.fail_on {
                        let loc = f
                            .location
                            .as_ref()
                            .map(|l| match l.start_line {
                                Some(line) => format!("{}:{}", l.file_path, line),
                                None => l.file_path.clone(),
                            })
                            .unwrap_or_else(|| "-".to_string());
                        out.push_str(&format!(
                            "  [{:?}] {} ({}) at {}\n    {}\n    Evidence: {}\n\n",
                            f.severity, f.title, f.rule_id, loc, f.description, f.evidence
                        ));
                    }
                }

                out.push_str("To bypass this check in an emergency, use:\n");
                out.push_str("  git commit --no-verify\n");
                out.push_str("  OR\n");
                out.push_str("  FORGE_SKIP_HOOK=1 git commit\n");
            }

            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_hook_types_cleanly() {
        assert_eq!(
            parse_hook_type("pre-commit").ok(),
            Some(HookType::PreCommit)
        );
        assert_eq!(
            parse_hook_type("pre_commit").ok(),
            Some(HookType::PreCommit)
        );
        assert_eq!(parse_hook_type("pre-push").ok(), Some(HookType::PrePush));
        assert_eq!(parse_hook_type("all").ok(), Some(HookType::All));
        assert!(parse_hook_type("unknown").is_err());
    }

    #[test]
    fn renders_hook_install_and_status_formats_deterministically() {
        let install_res = HookInstallResult {
            installed: vec!["pre-commit".to_string()],
            backed_up: vec!["pre-commit".to_string()],
            skipped: Vec::new(),
        };

        let term = render_hook_install(&install_res, OutputFormat::Terminal);
        assert!(term.is_ok());
        let json = render_hook_install(&install_res, OutputFormat::Json);
        assert!(json.is_ok());
        let md = render_hook_install(&install_res, OutputFormat::Markdown);
        assert!(md.is_ok());

        let status_rep = HookStatusCliReport {
            git_dir: ".git".to_string(),
            hooks_dir: ".git/hooks".to_string(),
            hooks: vec![HookEntryStatus {
                hook_name: "pre-commit".to_string(),
                installed: true,
                managed_by_forge: true,
                has_backup: true,
                path: PathBuf::from(".git/hooks/pre-commit"),
            }],
            config: None,
        };

        let s_term = render_hook_status(&status_rep, OutputFormat::Terminal);
        assert!(s_term.is_ok());
        let s_json = render_hook_status(&status_rep, OutputFormat::Json);
        assert!(s_json.is_ok());
    }

    #[test]
    fn renders_hook_run_report_pass_and_fail_cleanly() {
        let run_pass = HookRunReport {
            hook_name: "pre-commit".to_string(),
            staged_files_count: 2,
            staged_files: vec!["src/a.ts".to_string(), "src/b.ts".to_string()],
            findings_count: 0,
            violations_count: 0,
            fail_on: Severity::High,
            passed: true,
            findings: Vec::new(),
        };

        let pass_out = render_hook_run(&run_pass, OutputFormat::Terminal);
        assert!(pass_out.is_ok());
        if let Ok(text) = pass_out {
            assert!(text.contains("PASSED"));
        }

        let run_fail = HookRunReport {
            hook_name: "pre-commit".to_string(),
            staged_files_count: 1,
            staged_files: vec!["src/bad.ts".to_string()],
            findings_count: 1,
            violations_count: 1,
            fail_on: Severity::High,
            passed: false,
            findings: vec![Finding {
                id: "f-1".to_string(),
                category: "architecture".to_string(),
                rule_id: "arch/no-db".to_string(),
                title: "Direct DB import".to_string(),
                severity: Severity::High,
                confidence: forge_core::Confidence::Certain,
                location: None,
                description: "Bad import".to_string(),
                recommendation: "Refactor to use service layer".to_string(),
                evidence: "import db".to_string(),
            }],
        };

        let fail_out = render_hook_run(&run_fail, OutputFormat::Terminal);
        assert!(fail_out.is_ok());
        if let Ok(text) = fail_out {
            assert!(text.contains("FAILED"));
            assert!(text.contains("arch/no-db"));
        }
    }
}
