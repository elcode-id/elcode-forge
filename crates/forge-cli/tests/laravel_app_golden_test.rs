use std::error::Error;
use std::path::PathBuf;

use forge_cli::cli::OutputFormat;
use forge_cli::commands::dependencies::run_dependencies;
use forge_cli::commands::doctor::run_doctor;
use forge_cli::commands::explain::{execute_explain, render_explain_report};
use forge_cli::commands::review::{execute_review, render_review_report};
use forge_cli::commands::scan::{execute_scan, ScanOptions};
use forge_cli::formatters::render_repository_report;

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

#[test]
fn test_laravel_app_full_golden_suite() -> Result<(), Box<dyn Error>> {
    let root = fixture_dir("laravel-app");

    // 1. Doctor command
    let doctor_report = run_doctor(&root)?;
    assert!(doctor_report.repository_detected);

    let fw = doctor_report
        .checks
        .iter()
        .find(|c| c.label == "Language & framework detection")
        .ok_or("Missing framework check")?;
    assert!(
        fw.detail.contains("php"),
        "Expected PHP language in detail: {}",
        fw.detail
    );
    assert!(
        fw.detail.contains("laravel"),
        "Expected Laravel framework in detail: {}",
        fw.detail
    );
    assert!(
        fw.detail.contains("phpunit"),
        "Expected PHPUnit runner in detail: {}",
        fw.detail
    );

    let php_hygiene = doctor_report
        .checks
        .iter()
        .find(|c| c.label == "PHP manifest & lockfile hygiene")
        .ok_or("Missing PHP manifest hygiene check")?;
    assert!(
        php_hygiene.detail.contains("lockfile present: true"),
        "Expected lockfile present in PHP hygiene: {}",
        php_hygiene.detail
    );
    assert!(
        php_hygiene.detail.contains("unpinned: 1"),
        "Expected 1 unpinned dependency in PHP hygiene: {}",
        php_hygiene.detail
    );

    // 2. Scan command (polyglot graph, findings, formatting)
    let scan_opts = ScanOptions {
        repo_root: root.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };
    let (repo, exit_code) = execute_scan(&scan_opts)?;
    let health = repo.health.as_ref().ok_or("Missing health score")?;
    assert!(health.overall > 0);
    assert_eq!(health.dimensions.len(), 8);

    let total_weight: u8 = health.dimensions.iter().map(|d| d.weight_pct).sum();
    assert_eq!(
        total_weight, 100,
        "Health dimension weights must sum to 100%"
    );

    // Verify files & modules
    assert!(repo
        .files
        .iter()
        .any(|f| f.relative_path.contains("UserController.php")));
    assert!(repo
        .modules
        .iter()
        .any(|m| m.id == "app/Http/Controllers" || m.id == "app/Services"));

    // Verify circular dependency finding
    let has_cycle = repo.findings.iter().any(|f| {
        f.rule_id == "arch/no-circular-dependencies"
            && f.evidence.contains("app/Http/Controllers")
            && f.evidence.contains("app/Services")
    });
    assert!(
        has_cycle,
        "Expected circular dependency between app/Http/Controllers and app/Services"
    );

    // Verify unpinned dependency finding
    let has_unpinned = repo
        .findings
        .iter()
        .any(|f| f.rule_id == "deps/unpinned-version" && f.evidence.contains("guzzlehttp/guzzle"));
    assert!(
        has_unpinned,
        "Expected unpinned dependency finding for guzzlehttp/guzzle"
    );

    // Verify multi-format renderers
    let terminal_out = render_repository_report(&repo, OutputFormat::Terminal)?;
    assert!(terminal_out.contains("php") || terminal_out.contains("laravel"));
    let sarif_out = render_repository_report(&repo, OutputFormat::Sarif)?;
    assert!(sarif_out.contains("\"$schema\""));
    let md_out = render_repository_report(&repo, OutputFormat::Markdown)?;
    assert!(md_out.contains("# ELCODE Forge Report"));

    // 3. Review command
    let synthetic_diff = r#"diff --git a/app/Http/Controllers/UserController.php b/app/Http/Controllers/UserController.php
--- a/app/Http/Controllers/UserController.php
+++ b/app/Http/Controllers/UserController.php
@@ -10,3 +10,4 @@
     // update controller method
"#;
    let (review_diff, _) = execute_review(&root, Some(synthetic_diff), false)?;
    assert!(review_diff
        .impact
        .changed_files
        .iter()
        .any(|f| f.path.contains("UserController.php")));
    let review_json = render_review_report(&review_diff, OutputFormat::Json)?;
    assert!(review_json.contains("\"risk\""));

    // 4. Explain command (repo & module level)
    let explain_repo = execute_explain(&root, None)?;
    assert_eq!(explain_repo.target, ".");

    let explain_mod = execute_explain(&root, Some("app/Http/Controllers"))?;
    assert_eq!(explain_mod.target, "app/Http/Controllers");
    assert!(explain_mod
        .files
        .iter()
        .any(|f| f.contains("UserController.php")));
    assert!(explain_mod.imports.iter().any(|imp| imp == "app/Services"));
    let explain_json = render_explain_report(&explain_mod, OutputFormat::Json)?;
    assert!(explain_json.contains("\"target\": \"app/Http/Controllers\""));

    // 5. Dependencies command
    let deps_json = run_dependencies(&root, OutputFormat::Json)?;
    assert!(deps_json.contains("\"laravel/framework\""));
    assert!(deps_json.contains("\"guzzlehttp/guzzle\""));
    assert!(deps_json.to_lowercase().contains("composer"));

    let deps_md = run_dependencies(&root, OutputFormat::Markdown)?;
    assert!(deps_md.contains("`laravel/framework`"));
    assert!(deps_md.contains("`guzzlehttp/guzzle`"));

    let _ = exit_code;
    Ok(())
}
