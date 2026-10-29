use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_cli::cli::OutputFormat;
use forge_cli::commands::policy::{
    compute_policy_exit_code, execute_policy_check, render_policy_report, run_policy_check,
};
use forge_cli::exit_code::ExitCode;
use forge_core::ir::Severity;
use forge_core::policy::{PolicyReport, PolicyVerdict, PolicyWaiver};

fn create_temp_project(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!("forge_test_policy_cli_{name}"));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir)?;
    Ok(temp_dir)
}

#[test]
fn test_policy_check_compliant_project_exit_code_zero() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_project("compliant")?;

    // Create minimal clean package.json
    fs::write(
        temp_dir.join("package.json"),
        r#"{"name": "compliant-app", "version": "1.0.0", "dependencies": {}}"#,
    )?;

    // Create compliant policy
    let policy_path = temp_dir.join("policy.yaml");
    fs::write(
        &policy_path,
        r#"
name: "compliant-policy"
version: 1
constraints:
  min_health_score: 50
  enforce_secret_zero_tolerance: true
"#,
    )?;

    let exit_code = run_policy_check(&temp_dir, Some(&policy_path), OutputFormat::Terminal, None);
    assert_eq!(exit_code, ExitCode::Success);

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_policy_check_violation_exit_code_four() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_project("violation")?;

    fs::write(
        temp_dir.join("package.json"),
        r#"{"name": "violation-app", "version": "1.0.0", "dependencies": {}}"#,
    )?;

    // Policy with unreachable health requirement
    let policy_path = temp_dir.join("strict-policy.yaml");
    fs::write(
        &policy_path,
        r#"
name: "impossible-policy"
version: 1
constraints:
  min_health_score: 100
  disallowed_severities:
    - critical
    - high
"#,
    )?;

    let report = execute_policy_check(&temp_dir, Some(&policy_path))?;
    assert_eq!(report.verdict, PolicyVerdict::Violation);
    assert!(report.is_violation());

    let exit_code = run_policy_check(&temp_dir, Some(&policy_path), OutputFormat::Json, None);
    assert_eq!(exit_code, ExitCode::SecurityPolicyFailure);

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_policy_check_fallback_to_eforge_config_yaml() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_project("inline_config")?;

    fs::write(
        temp_dir.join("package.json"),
        r#"{"name": "inline-app", "version": "1.0.0", "dependencies": {}}"#,
    )?;

    fs::write(
        temp_dir.join("eforge.config.yaml"),
        r#"
version: 1
project_name: "inline-policy-test"

policy:
  name: "embedded-gate"
  version: 1
  constraints:
    min_health_score: 60
"#,
    )?;

    // Run without passing explicit --policy path
    let report = execute_policy_check(&temp_dir, None)?;
    assert_eq!(report.policy_name, "embedded-gate");

    let exit_code = run_policy_check(&temp_dir, None, OutputFormat::Json, None);
    assert_eq!(exit_code, ExitCode::Success);

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_policy_check_missing_policy_returns_config_error() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_project("missing_policy")?;

    fs::write(
        temp_dir.join("package.json"),
        r#"{"name": "no-policy-app", "version": "1.0.0", "dependencies": {}}"#,
    )?;

    let exit_code = run_policy_check(&temp_dir, None, OutputFormat::Terminal, None);
    assert_eq!(exit_code, ExitCode::ConfigurationError);

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_compute_policy_exit_code() {
    let report_pass = PolicyReport {
        verdict: PolicyVerdict::Passed,
        policy_name: "test".to_string(),
        passed_checks: vec!["all clear".to_string()],
        warnings: Vec::new(),
        violations: Vec::new(),
        active_waivers: Vec::new(),
    };
    assert_eq!(
        compute_policy_exit_code(&report_pass, None),
        ExitCode::Success
    );
    assert_eq!(
        compute_policy_exit_code(&report_pass, Some(Severity::High)),
        ExitCode::Success
    );

    let report_violation = PolicyReport {
        verdict: PolicyVerdict::Violation,
        policy_name: "test".to_string(),
        passed_checks: Vec::new(),
        warnings: Vec::new(),
        violations: vec!["disallowed finding".to_string()],
        active_waivers: Vec::new(),
    };
    assert_eq!(
        compute_policy_exit_code(&report_violation, None),
        ExitCode::SecurityPolicyFailure
    );

    let report_warn = PolicyReport {
        verdict: PolicyVerdict::Warning,
        policy_name: "test".to_string(),
        passed_checks: Vec::new(),
        warnings: vec!["waiver applied".to_string()],
        violations: Vec::new(),
        active_waivers: vec![PolicyWaiver {
            rule_id: "rule1".to_string(),
            path_pattern: None,
            expires_at: None,
            justification: "test".to_string(),
        }],
    };
    // Warning without fail_on -> Success (0)
    assert_eq!(
        compute_policy_exit_code(&report_warn, None),
        ExitCode::Success
    );
    // Warning with fail_on medium -> FindingsAboveThreshold (1)
    assert_eq!(
        compute_policy_exit_code(&report_warn, Some(Severity::Medium)),
        ExitCode::FindingsAboveThreshold
    );
    // Warning with fail_on high -> Success (0)
    assert_eq!(
        compute_policy_exit_code(&report_warn, Some(Severity::High)),
        ExitCode::Success
    );
}

#[test]
fn test_render_policy_report_formats() -> Result<(), Box<dyn Error>> {
    let report = PolicyReport {
        verdict: PolicyVerdict::Warning,
        policy_name: "render-test".to_string(),
        passed_checks: vec!["Check A passed".to_string()],
        warnings: vec!["Warning B occurred".to_string()],
        violations: vec!["Violation C detected".to_string()],
        active_waivers: vec![PolicyWaiver {
            rule_id: "rule-x".to_string(),
            path_pattern: Some("legacy/**".to_string()),
            expires_at: Some("2027-01-01T00:00:00Z".to_string()),
            justification: "Refactor ticket".to_string(),
        }],
    };

    // JSON
    let json = render_policy_report(&report, OutputFormat::Json)?;
    assert!(json.contains("\"policy_name\": \"render-test\""));
    assert!(json.contains("\"verdict\": \"warning\""));

    // Markdown
    let md = render_policy_report(&report, OutputFormat::Markdown)?;
    assert!(md.contains("# Policy-as-Code Gate Report: render-test"));
    assert!(md.contains("### Policy Violations"));
    assert!(md.contains("### Active Waivers"));
    assert!(md.contains("`rule-x`"));

    // Terminal
    let term = render_policy_report(&report, OutputFormat::Terminal)?;
    assert!(term.contains("ELCODE Forge Policy Gate — render-test"));
    assert!(term.contains("WARNING"));
    assert!(term.contains("Violations:"));
    assert!(term.contains("Passed Checks:"));

    Ok(())
}
