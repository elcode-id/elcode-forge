use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_cli::cli::OutputFormat;
use forge_cli::commands::audit::{execute_audit, render_audit_report, run_audit};
use forge_cli::exit_code::ExitCode;
use forge_security::compliance::ComplianceStandard;

fn create_temp_project(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join(format!("forge_test_audit_cli_{name}"));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir)?;
    Ok(temp_dir)
}

#[test]
fn test_audit_cli_clean_repository_pass() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_project("clean")?;

    fs::write(
        temp_dir.join("package.json"),
        r#"{"name": "audit-clean-app", "version": "1.0.0", "dependencies": {}}"#,
    )?;

    // Run SOC 2 audit
    let code_soc2 = run_audit(
        &temp_dir,
        ComplianceStandard::Soc2,
        OutputFormat::Terminal,
        None,
    );
    assert_eq!(code_soc2, ExitCode::Success);

    // Run OWASP audit
    let code_owasp = run_audit(
        &temp_dir,
        ComplianceStandard::Owasp,
        OutputFormat::Terminal,
        None,
    );
    assert_eq!(code_owasp, ExitCode::Success);

    // Run ISO 27001 audit
    let code_iso = run_audit(
        &temp_dir,
        ComplianceStandard::Iso27001,
        OutputFormat::Terminal,
        None,
    );
    assert_eq!(code_iso, ExitCode::Success);

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_audit_cli_vulnerable_project_fail() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_project("vulnerable")?;

    fs::write(
        temp_dir.join("package.json"),
        r#"{"name": "audit-vuln-app", "version": "1.0.0", "dependencies": {}}"#,
    )?;

    // Introduce a secret in source code
    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(
        src_dir.join("config.ts"),
        r#"export const AWS_SECRET = "AKIA1111111111111111";"#,
    )?;

    // SOC 2 audit must fail due to CC6.1
    let code = run_audit(
        &temp_dir,
        ComplianceStandard::Soc2,
        OutputFormat::Json,
        None,
    );
    assert_eq!(code, ExitCode::SecurityPolicyFailure);

    // Report inspection
    let report = execute_audit(&temp_dir, ComplianceStandard::Soc2)?;
    assert!(!report.overall_passed);
    let cc6_1 = report
        .clauses
        .iter()
        .find(|c| c.clause_id == "CC6.1")
        .ok_or("CC6.1 missing")?;
    assert_eq!(
        cc6_1.status,
        forge_security::compliance::ComplianceClauseStatus::Fail
    );

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_audit_cli_output_file_writing() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_project("output_file")?;

    fs::write(
        temp_dir.join("package.json"),
        r#"{"name": "audit-file-app", "version": "1.0.0", "dependencies": {}}"#,
    )?;

    let out_file = temp_dir.join("reports").join("audit.md");
    let code = run_audit(
        &temp_dir,
        ComplianceStandard::Owasp,
        OutputFormat::Markdown,
        Some(&out_file),
    );
    assert_eq!(code, ExitCode::Success);

    assert!(out_file.exists());
    let content = fs::read_to_string(&out_file)?;
    assert!(content.contains("# Compliance Audit Report: OWASP Top 10 (2021)"));
    assert!(content.contains("A01:2021"));
    assert!(content.contains("A02:2021"));
    assert!(content.contains("A03:2021"));
    assert!(content.contains("A06:2021"));

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

#[test]
fn test_render_audit_report_formats() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_project("render_formats")?;

    fs::write(
        temp_dir.join("package.json"),
        r#"{"name": "audit-render-app", "version": "1.0.0", "dependencies": {}}"#,
    )?;

    let report = execute_audit(&temp_dir, ComplianceStandard::Iso27001)?;

    let json = render_audit_report(&report, OutputFormat::Json)?;
    assert!(json.contains("\"standard\": \"iso27001\""));
    assert!(json.contains("\"overall_passed\": true"));

    let md = render_audit_report(&report, OutputFormat::Markdown)?;
    assert!(md.contains("# Compliance Audit Report: ISO/IEC 27001:2022"));
    assert!(md.contains("Control Clause Evaluation"));

    let term = render_audit_report(&report, OutputFormat::Terminal)?;
    assert!(term.contains("ELCODE Forge Compliance Audit — ISO/IEC 27001:2022"));
    assert!(term.contains("Status:            PASSED"));

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}
