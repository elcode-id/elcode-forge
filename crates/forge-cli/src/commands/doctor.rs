//! `eforge doctor` command — fast (<5s) repository diagnostic check.

use std::path::Path;
use std::time::Instant;

use forge_analyzer::{package_json::parse_package_json, scanner::scan_repository};
use forge_core::{config::load_config, ForgeError};
use forge_security::SecurityScanner;
use serde::{Deserialize, Serialize};

use crate::cli::OutputFormat;

/// Individual diagnostic check result emitted by `eforge doctor`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorCheck {
    /// Human-readable check name.
    pub label: String,
    /// True if the check passed without critical errors.
    pub passed: bool,
    /// True if the check produced a non-fatal warning.
    pub warning: bool,
    /// Actionable diagnostic detail.
    pub detail: String,
}

/// Complete diagnostic report emitted by `eforge doctor`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorReport {
    /// Whether a valid project directory with source files was detected.
    pub repository_detected: bool,
    /// Ordered list of diagnostic checks.
    pub checks: Vec<DoctorCheck>,
    /// Total count of checks with `warning == true` or `passed == false`.
    pub warnings_count: usize,
    /// Total execution wall-clock time in milliseconds.
    pub duration_ms: u64,
}

/// Executes a fast (<5s) diagnostic check on `repo_root`.
pub fn run_doctor(repo_root: &Path) -> Result<DoctorReport, ForgeError> {
    let start = Instant::now();
    let config = load_config(repo_root)?;
    let discovery = scan_repository(repo_root, &config.ignore_patterns)?;

    let repository_detected = !discovery.files.is_empty();
    let mut checks = Vec::new();

    // Check 1: Repository & File Inventory
    checks.push(DoctorCheck {
        label: "Repository inventory".to_string(),
        passed: repository_detected,
        warning: !repository_detected,
        detail: format!("Discovered {} source/config file(s)", discovery.files.len()),
    });

    // Check 2: Languages & Frameworks
    let has_langs = !discovery.languages.is_empty();
    checks.push(DoctorCheck {
        label: "Language & framework detection".to_string(),
        passed: has_langs,
        warning: !has_langs,
        detail: format!(
            "Languages: [{}], Frameworks: [{}]",
            discovery.languages.join(", "),
            discovery.frameworks.join(", ")
        ),
    });

    // Check 3: Package Manager & Manifest Hygiene
    let pkg_json_path = repo_root.join("package.json");
    if pkg_json_path.exists() {
        let manifest = parse_package_json(repo_root)?;
        let has_lockfile = !discovery.package_managers.is_empty()
            && (repo_root.join("pnpm-lock.yaml").exists()
                || repo_root.join("package-lock.json").exists()
                || repo_root.join("yarn.lock").exists()
                || repo_root.join("bun.lockb").exists());
        let dup_warn = !manifest.duplicate_packages.is_empty();
        checks.push(DoctorCheck {
            label: "Node.js manifest & lockfile hygiene".to_string(),
            passed: true,
            warning: !has_lockfile || dup_warn,
            detail: format!(
                "{} dependency(ies) declared, lockfile present: {}, duplicates: {}",
                manifest.dependencies.len(),
                has_lockfile,
                manifest.duplicate_packages.len()
            ),
        });
    } else if repo_root.join("Cargo.toml").exists() {
        let has_cargo_lock = repo_root.join("Cargo.lock").exists();
        checks.push(DoctorCheck {
            label: "Cargo manifest & lockfile hygiene".to_string(),
            passed: true,
            warning: !has_cargo_lock,
            detail: format!("Cargo.toml present, Cargo.lock present: {has_cargo_lock}"),
        });
    } else {
        checks.push(DoctorCheck {
            label: "Package manifest".to_string(),
            passed: true,
            warning: true,
            detail: "No package.json found at repository root".to_string(),
        });
    }

    // Check 4: Test Suite Presence
    let test_files_count = discovery.files.iter().filter(|f| f.is_test).count();
    checks.push(DoctorCheck {
        label: "Automated test suite".to_string(),
        passed: test_files_count > 0,
        warning: test_files_count == 0,
        detail: format!("Found {} test file(s)", test_files_count),
    });

    // Check 5: Security & Secret Hygiene
    let security_findings = SecurityScanner::scan_repository_files(repo_root, &discovery.files)?;
    let has_sec_issues = !security_findings.is_empty();
    checks.push(DoctorCheck {
        label: "Security & secret hygiene".to_string(),
        passed: !has_sec_issues,
        warning: has_sec_issues,
        detail: if has_sec_issues {
            format!(
                "Detected {} security/secret finding(s)",
                security_findings.len()
            )
        } else {
            "No hardcoded secrets or unsafe patterns detected".to_string()
        },
    });

    // Check 6: Architectural & Quality Rules Hygiene
    let rule_engine = forge_rules::RuleEngine::from_config(repo_root, &config)?;
    let active_rules_count = rule_engine.rules().iter().filter(|r| r.enabled).count()
        + rule_engine
            .declarative_rules()
            .iter()
            .filter(|r| r.enabled)
            .count();
    let packs_display = if config.rule_packs.is_empty() {
        "recommended".to_string()
    } else {
        config.rule_packs.join(", ")
    };
    checks.push(DoctorCheck {
        label: "Architectural & quality rules".to_string(),
        passed: true,
        warning: false,
        detail: format!(
            "Loaded {} active rule(s) (packs: [{}], custom: {})",
            active_rules_count,
            packs_display,
            rule_engine.declarative_rules().len()
        ),
    });

    let warnings_count = checks
        .iter()
        .filter(|check| check.warning || !check.passed)
        .count();
    let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

    Ok(DoctorReport {
        repository_detected,
        checks,
        warnings_count,
        duration_ms,
    })
}

/// Formats a [`DoctorReport`] according to the requested [`OutputFormat`].
pub fn render_doctor_report(
    report: &DoctorReport,
    format: OutputFormat,
) -> Result<String, ForgeError> {
    match format {
        OutputFormat::Json | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report).map_err(|err| ForgeError::Serialization {
                message: err.to_string(),
            })
        }
        OutputFormat::Markdown | OutputFormat::Github => {
            let mut md = String::new();
            md.push_str("# ELCODE Forge Doctor\n\n");
            for check in &report.checks {
                let icon = if !check.passed {
                    "FAIL"
                } else if check.warning {
                    "WARN"
                } else {
                    "OK"
                };
                md.push_str(&format!(
                    "- **[{}]** **{}** — {}\n",
                    icon, check.label, check.detail
                ));
            }
            md.push_str(&format!(
                "\n_Completed in {}ms with {} warning(s)._\n",
                report.duration_ms, report.warnings_count
            ));
            Ok(md)
        }
        OutputFormat::Terminal => {
            let mut out = String::new();
            out.push_str("ELCODE Forge Doctor\n");
            out.push_str("-------------------\n");
            for check in &report.checks {
                let status = if !check.passed {
                    "[FAIL]"
                } else if check.warning {
                    "[WARN]"
                } else {
                    "[ OK ]"
                };
                out.push_str(&format!("{} {}: {}\n", status, check.label, check.detail));
            }
            out.push_str(&format!(
                "\nSummary: {} warning(s) in {}ms\n",
                report.warnings_count, report.duration_ms
            ));
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{render_doctor_report, run_doctor};
    use crate::cli::OutputFormat;

    fn fixture_next_app_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app")
    }

    #[test]
    fn diagnoses_fixture_next_app_in_under_five_seconds() {
        let root = fixture_next_app_dir();
        let report_res = run_doctor(&root);
        assert!(report_res.is_ok());

        if let Ok(report) = report_res {
            assert!(report.repository_detected);
            assert!(report.duration_ms < 5_000);
            assert_eq!(report.checks.len(), 6);

            let json_res = render_doctor_report(&report, OutputFormat::Json);
            assert!(json_res.is_ok());
            if let Ok(json_text) = json_res {
                assert!(json_text.contains("\"repository_detected\": true"));
            }
        }
    }
}
