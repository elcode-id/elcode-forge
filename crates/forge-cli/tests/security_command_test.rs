use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_cli::cli::OutputFormat;
use forge_cli::commands::scan::{execute_scan, ScanOptions};
use forge_cli::commands::security::run_security;
use forge_cli::exit_code::ExitCode;

#[test]
fn test_security_scan_on_vulnerable_fixture() -> Result<(), Box<dyn Error>> {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("vulnerable-ts-app");

    let (rendered, exit_code) =
        run_security(&fixture_dir, OutputFormat::Terminal, false, true, None)?;

    assert!(!rendered.is_empty());
    // vulnerable-ts-app has critical secrets and unsafe patterns
    assert!(rendered.contains("Hardcoded secret") || rendered.contains("eval"));
    // Default config has fail_on_severity: High or None
    assert!(exit_code == ExitCode::SecurityPolicyFailure || exit_code == ExitCode::Success);
    Ok(())
}

#[test]
fn test_security_audit_with_vulnerabilities_and_sarif_export() -> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join("forge_test_cli_security_audit");
    let _ = fs::create_dir_all(&temp_dir);

    // Create a package.json containing lodash 4.17.15 (CVE-2020-8203)
    let package_json = r#"{
      "name": "vulnerable-audit-test",
      "version": "1.0.0",
      "dependencies": {
        "lodash": "4.17.15"
      }
    }"#;
    fs::write(temp_dir.join("package.json"), package_json)?;

    // 1. Terminal format with --audit
    let (terminal_output, exit_code) =
        run_security(&temp_dir, OutputFormat::Terminal, true, true, None)?;

    assert!(terminal_output.contains("Dependency Vulnerability Audit"));
    assert!(terminal_output.contains("High:     1"));
    assert!(terminal_output.contains("lodash@4.17.15"));
    assert!(terminal_output.contains("CVE-2020-8203"));
    assert!(terminal_output.contains("4.17.19"));
    assert_eq!(exit_code, ExitCode::Success);

    // 2. Policy failure with --fail-on high
    let (_, fail_code) = run_security(&temp_dir, OutputFormat::Terminal, true, true, Some("high"))?;
    assert_eq!(fail_code, ExitCode::SecurityPolicyFailure);

    // 3. Passing policy with --fail-on critical (lodash is High severity, so Critical threshold passes)
    let (_, pass_code) = run_security(
        &temp_dir,
        OutputFormat::Terminal,
        true,
        true,
        Some("critical"),
    )?;
    assert_eq!(pass_code, ExitCode::Success);

    // 4. JSON format
    let (json_output, _) = run_security(&temp_dir, OutputFormat::Json, true, true, None)?;
    assert!(json_output.starts_with('['));
    assert!(json_output.contains("security/vulnerability/cve-2020-8203"));

    // 5. SARIF format
    let (sarif_output, _) = run_security(&temp_dir, OutputFormat::Sarif, true, true, None)?;
    assert!(sarif_output.contains("https://json.schemastore.org/sarif-2.1.0.json"));
    assert!(sarif_output.contains("security/vulnerable-dependency"));
    assert!(sarif_output.contains("security-severity"));
    assert!(sarif_output.contains("https://nvd.nist.gov/vuln/detail/CVE-2020-8203"));

    // 6. Verify scan pipeline includes vulnerability findings
    let scan_opts = ScanOptions {
        repo_root: temp_dir.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };
    let (repo, _) = execute_scan(&scan_opts)?;
    assert!(repo
        .findings
        .iter()
        .any(|f| f.rule_id == "security/vulnerable-dependency"));
    assert!(repo.health.is_some());

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}
