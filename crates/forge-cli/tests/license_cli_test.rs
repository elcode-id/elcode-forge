use std::error::Error;
use std::fs;

use forge_cli::cli::OutputFormat;
use forge_cli::commands::dependencies::run_dependencies;
use forge_cli::commands::scan::{execute_scan, ScanOptions};
use forge_cli::exit_code::ExitCode;

#[test]
fn test_license_cli_and_scan_integration() -> Result<(), Box<dyn Error>> {
    let temp_dir = std::env::temp_dir().join("forge_test_cli_license_integration");
    let _ = fs::create_dir_all(&temp_dir);

    // Write a project manifest with a strong copyleft package and a permissive package
    let package_json = r#"{
      "name": "license-cli-test",
      "version": "1.0.0",
      "license": "MIT",
      "dependencies": {
        "lodash": "4.17.21",
        "copyleft-dep": "1.0.0"
      }
    }"#;
    fs::write(temp_dir.join("package.json"), package_json)?;

    // Create node_modules for copyleft-dep declaring GPL-3.0
    let copyleft_dir = temp_dir.join("node_modules").join("copyleft-dep");
    fs::create_dir_all(&copyleft_dir)?;
    fs::write(
        copyleft_dir.join("package.json"),
        r#"{"name": "copyleft-dep", "version": "1.0.0", "license": "GPL-3.0"}"#,
    )?;

    // Configure eforge.config.yaml with disallow_copyleft: true
    let config_yaml = r#"version: 1
project_name: "license-cli-test"
license:
  disallow_copyleft: true
"#;
    fs::write(temp_dir.join("eforge.config.yaml"), config_yaml)?;

    // 1. Run eforge dependencies (Terminal format)
    let term_deps = run_dependencies(&temp_dir, OutputFormat::Terminal)?;
    assert!(term_deps.contains("Project License:      MIT (package.json)"));
    assert!(term_deps.contains("license/disallowed-copyleft"));

    // 2. Run eforge dependencies (Markdown format)
    let md_deps = run_dependencies(&temp_dir, OutputFormat::Markdown)?;
    assert!(md_deps.contains("| Package | Ecosystem | Version Req | Resolved | Type |"));
    assert!(md_deps.contains("### Open-Source License Inventory"));
    assert!(md_deps.contains("`copyleft-dep`"));
    assert!(md_deps.contains("`GPL-3.0`"));

    // 3. Run eforge scan and verify license finding integration
    let scan_opts = ScanOptions {
        repo_root: temp_dir.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };
    let (repo, exit_code) = execute_scan(&scan_opts)?;
    assert!(repo
        .findings
        .iter()
        .any(|f| f.rule_id == "license/disallowed-copyleft"));

    // Default exit_code should be success because fail_on_severity is not configured
    assert_eq!(exit_code, ExitCode::Success);

    // 4. Update config with fail_on_severity: high
    let config_yaml_strict = r#"version: 1
project_name: "license-cli-test"
fail_on_severity: high
license:
  disallow_copyleft: true
"#;
    fs::write(temp_dir.join("eforge.config.yaml"), config_yaml_strict)?;
    let (_, strict_exit_code) = execute_scan(&scan_opts)?;
    // Since license/disallowed-copyleft is High severity and category "license", strict exit code should trigger FindingsAboveThreshold
    assert_eq!(strict_exit_code, ExitCode::FindingsAboveThreshold);

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}
