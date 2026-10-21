use std::error::Error;
use std::path::PathBuf;

use forge_cli::cli::OutputFormat;
use forge_cli::commands::doctor::run_doctor;
use forge_cli::commands::scan::{execute_scan, ScanOptions};
use forge_cli::formatters::render_repository_report;

fn fixture_fastapi_app_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("fastapi-app")
}

#[test]
fn test_fastapi_app_doctor_diagnostics() -> Result<(), Box<dyn Error>> {
    let root = fixture_fastapi_app_dir();
    let report = run_doctor(&root)?;

    assert!(report.repository_detected);
    assert!(report.duration_ms < 5000);

    // 1. Language & framework detection
    let fw_check = report
        .checks
        .iter()
        .find(|c| c.label == "Language & framework detection")
        .ok_or("Missing framework check")?;
    assert!(fw_check.detail.contains("python"));
    assert!(fw_check.detail.contains("fastapi"));
    assert!(fw_check.detail.contains("pytest"));

    // 2. Python manifest & lockfile hygiene check
    let py_check = report
        .checks
        .iter()
        .find(|c| c.label == "Python manifest & lockfile hygiene")
        .ok_or("Missing Python manifest check")?;
    assert!(py_check.warning); // 1 unpinned dependency
    assert!(py_check.detail.contains("unpinned: 1"));
    assert!(py_check.detail.contains("lockfile present: true"));

    // 3. Automated test suite check
    let test_check = report
        .checks
        .iter()
        .find(|c| c.label == "Automated test suite")
        .ok_or("Missing test check")?;
    assert!(test_check.passed);
    assert!(!test_check.warning);

    // 4. Security & secret hygiene check (detects pickle.loads)
    let sec_check = report
        .checks
        .iter()
        .find(|c| c.label == "Security & secret hygiene")
        .ok_or("Missing security check")?;
    assert!(sec_check.warning);
    assert!(sec_check
        .detail
        .contains("Detected 1 security/secret finding(s)"));

    Ok(())
}

#[test]
fn test_fastapi_app_scan_signals_and_determinism() -> Result<(), Box<dyn Error>> {
    let root = fixture_fastapi_app_dir();
    let opts = ScanOptions {
        repo_root: root.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };

    let (repo1, _) = execute_scan(&opts)?;
    let (repo2, _) = execute_scan(&opts)?;

    // Strict byte determinism
    assert_eq!(repo1, repo2);

    assert_eq!(repo1.detected_languages, vec!["python"]);
    assert!(repo1.detected_frameworks.contains(&"fastapi".to_string()));
    assert!(repo1.detected_frameworks.contains(&"pytest".to_string()));
    assert!(repo1.package_managers.contains(&"poetry".to_string()));

    let rule_ids: Vec<&str> = repo1.findings.iter().map(|f| f.rule_id.as_str()).collect();

    // Verify intentional signals are all detected
    assert!(
        rule_ids.contains(&"arch/no-circular-dependencies"),
        "Must detect circular dependency between app/routers and app/services"
    );
    assert!(
        rule_ids.contains(&"security/python-pickle-deserialization"),
        "Must detect unsafe pickle deserialization"
    );
    assert!(
        rule_ids.contains(&"deps/unpinned-version"),
        "Must detect unpinned requests dependency"
    );

    // Verify report formatting
    let json = render_repository_report(&repo1, OutputFormat::Json)?;
    assert!(json.contains("fastapi-app"));
    assert!(json.contains("arch/no-circular-dependencies"));
    assert!(json.contains("security/python-pickle-deserialization"));

    Ok(())
}
