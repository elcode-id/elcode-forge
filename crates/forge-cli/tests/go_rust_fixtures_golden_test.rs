use std::error::Error;
use std::path::PathBuf;

use forge_cli::cli::OutputFormat;
use forge_cli::commands::doctor::run_doctor;
use forge_cli::commands::graph::run_graph;
use forge_cli::commands::scan::{execute_scan, ScanOptions};
use forge_cli::formatters::render_repository_report;

fn fixture_go_service_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("go-service")
}

fn fixture_rust_cli_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("rust-cli")
}

#[test]
fn test_go_service_doctor_diagnostics() -> Result<(), Box<dyn Error>> {
    let root = fixture_go_service_dir();
    let report = run_doctor(&root)?;

    assert!(report.repository_detected);
    assert!(report.duration_ms < 5000);

    // 1. Language & framework detection
    let fw_check = report
        .checks
        .iter()
        .find(|c| c.label == "Language & framework detection")
        .ok_or("Missing framework check")?;
    assert!(fw_check.detail.contains("go"));
    assert!(fw_check.detail.contains("gin"));

    // 2. Go manifest & lockfile hygiene check
    let go_check = report
        .checks
        .iter()
        .find(|c| c.label == "Go manifest & lockfile hygiene")
        .ok_or("Missing Go manifest check")?;
    assert!(!go_check.warning);
    assert!(go_check.detail.contains("go.sum present: true"));

    // 3. Automated test suite check
    let test_check = report
        .checks
        .iter()
        .find(|c| c.label == "Automated test suite")
        .ok_or("Missing test check")?;
    assert!(test_check.passed);
    assert!(!test_check.warning);

    Ok(())
}

#[test]
fn test_go_service_scan_signals_and_determinism() -> Result<(), Box<dyn Error>> {
    let root = fixture_go_service_dir();
    let opts = ScanOptions {
        repo_root: root.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };

    let (repo1, _) = execute_scan(&opts)?;
    let (repo2, _) = execute_scan(&opts)?;

    // Strict byte determinism
    assert_eq!(repo1, repo2);

    assert_eq!(repo1.detected_languages, vec!["go"]);
    assert!(repo1.detected_frameworks.contains(&"gin".to_string()));
    assert!(repo1.package_managers.contains(&"go".to_string()));

    let rule_ids: Vec<&str> = repo1.findings.iter().map(|f| f.rule_id.as_str()).collect();

    // Verify circular dependency between internal/handlers and internal/services
    assert!(
        rule_ids.contains(&"arch/no-circular-dependencies"),
        "Must detect circular dependency between internal/handlers and internal/services"
    );

    // Verify architecture graph exports
    let graph_md = run_graph(&root, OutputFormat::Markdown)?;
    assert!(graph_md.contains("```mermaid"));
    assert!(graph_md.contains("internal/handlers <-> internal/services"));

    // Verify report formatting
    let json = render_repository_report(&repo1, OutputFormat::Json)?;
    assert!(json.contains("go-service"));
    assert!(json.contains("arch/no-circular-dependencies"));

    Ok(())
}

#[test]
fn test_rust_cli_doctor_diagnostics() -> Result<(), Box<dyn Error>> {
    let root = fixture_rust_cli_dir();
    let report = run_doctor(&root)?;

    assert!(report.repository_detected);
    assert!(report.duration_ms < 5000);

    // 1. Language & framework detection
    let fw_check = report
        .checks
        .iter()
        .find(|c| c.label == "Language & framework detection")
        .ok_or("Missing framework check")?;
    assert!(fw_check.detail.contains("rust"));

    // 2. Rust manifest & lockfile hygiene check
    let rust_check = report
        .checks
        .iter()
        .find(|c| c.label == "Rust manifest & lockfile hygiene")
        .ok_or("Missing Rust manifest check")?;
    assert!(rust_check.warning); // 1 wildcard dependency (log = "*")
    assert!(rust_check.detail.contains("lockfile present: true"));
    assert!(rust_check.detail.contains("wildcards: 1"));

    // 3. Automated test suite check
    let test_check = report
        .checks
        .iter()
        .find(|c| c.label == "Automated test suite")
        .ok_or("Missing test check")?;
    assert!(test_check.passed);
    assert!(!test_check.warning);

    Ok(())
}

#[test]
fn test_rust_cli_scan_signals_and_determinism() -> Result<(), Box<dyn Error>> {
    let root = fixture_rust_cli_dir();
    let opts = ScanOptions {
        repo_root: root.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };

    let (repo1, _) = execute_scan(&opts)?;
    let (repo2, _) = execute_scan(&opts)?;

    // Strict byte determinism
    assert_eq!(repo1, repo2);

    assert_eq!(repo1.detected_languages, vec!["rust"]);
    assert!(repo1.package_managers.contains(&"cargo".to_string()));

    let rule_ids: Vec<&str> = repo1.findings.iter().map(|f| f.rule_id.as_str()).collect();

    // Verify wildcard dependency is detected
    assert!(
        rule_ids.contains(&"deps/unpinned-version"),
        "Must detect wildcard dependency on log = '*'"
    );

    // Verify module structure
    let mod_ids: Vec<&str> = repo1.modules.iter().map(|m| m.id.as_str()).collect();
    assert!(mod_ids.contains(&"src/commands"));
    assert!(mod_ids.contains(&"src/utils"));

    // Verify architecture graph exports
    let graph_md = run_graph(&root, OutputFormat::Markdown)?;
    assert!(graph_md.contains("```mermaid"));
    assert!(graph_md.contains("src/commands"));
    assert!(graph_md.contains("src/utils"));

    // Verify report formatting
    let json = render_repository_report(&repo1, OutputFormat::Json)?;
    assert!(json.contains("rust-cli"));
    assert!(json.contains("deps/unpinned-version"));

    Ok(())
}
