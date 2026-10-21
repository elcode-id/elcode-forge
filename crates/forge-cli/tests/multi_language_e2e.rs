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

// ---------------------------------------------------------------------------
// 1. Next.js / TypeScript Fixture E2E Test Suite
// ---------------------------------------------------------------------------

#[test]
fn test_next_app_full_e2e_suite() -> Result<(), Box<dyn Error>> {
    let root = fixture_dir("next-app");

    // 1. Doctor command
    let doctor_report = run_doctor(&root)?;
    assert!(doctor_report.repository_detected);
    let fw = doctor_report
        .checks
        .iter()
        .find(|c| c.label == "Language & framework detection")
        .ok_or("Missing framework check")?;
    assert!(fw.detail.contains("typescript") || fw.detail.contains("javascript"));
    assert!(fw.detail.contains("nextjs") || fw.detail.contains("react"));

    // 2. Scan command (deterministic JSON + formatting)
    let scan_opts = ScanOptions {
        repo_root: root.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };
    let (repo, exit_code) = execute_scan(&scan_opts)?;
    let health = repo.health.as_ref().ok_or("Missing health score")?;
    assert!(health.overall > 0);
    assert_eq!(health.dimensions.len(), 8);
    assert!(repo
        .files
        .iter()
        .any(|f| f.relative_path.contains("UserCard.tsx")));
    assert!(repo.modules.iter().any(|m| m.id.contains("src/components")));

    let terminal_out = render_repository_report(&repo, OutputFormat::Terminal)?;
    assert!(terminal_out.contains("ELCODE Forge Report"));
    let sarif_out = render_repository_report(&repo, OutputFormat::Sarif)?;
    assert!(sarif_out.contains("\"$schema\""));
    let md_out = render_repository_report(&repo, OutputFormat::Markdown)?;
    assert!(md_out.contains("# ELCODE Forge Report"));

    // 3. Review command (no diff + synthetic diff)
    let (review_clean, _) = execute_review(&root, None, false)?;
    assert!(review_clean.risk.score <= 100);

    let synthetic_diff = r#"diff --git a/src/components/UserCard.tsx b/src/components/UserCard.tsx
--- a/src/components/UserCard.tsx
+++ b/src/components/UserCard.tsx
@@ -1,3 +1,4 @@
+// test comment
"#;
    let (review_diff, _) = execute_review(&root, Some(synthetic_diff), false)?;
    assert!(review_diff
        .impact
        .changed_files
        .iter()
        .any(|f| f.path.contains("UserCard.tsx")));
    let review_json = render_review_report(&review_diff, OutputFormat::Json)?;
    assert!(review_json.contains("\"risk\""));
    let review_md = render_review_report(&review_diff, OutputFormat::Markdown)?;
    assert!(review_md.contains("ELCODE Forge Review Report"));

    // 4. Explain command (repo-level + module-level)
    let explain_repo = execute_explain(&root, None)?;
    assert_eq!(explain_repo.target, ".");
    assert!(!explain_repo.files.is_empty());

    let explain_mod = execute_explain(&root, Some("src/components"))?;
    assert_eq!(explain_mod.target, "src/components");
    assert!(explain_mod.files.iter().any(|f| f.contains("UserCard.tsx")));
    let explain_json = render_explain_report(&explain_mod, OutputFormat::Json)?;
    assert!(explain_json.contains("\"target\": \"src/components\""));

    // 5. Dependencies command (JSON + Markdown)
    let deps_json = run_dependencies(&root, OutputFormat::Json)?;
    assert!(deps_json.contains("\"react\""));
    assert!(deps_json.contains("\"ecosystem\": \"npm\""));
    let deps_md = run_dependencies(&root, OutputFormat::Markdown)?;
    assert!(deps_md.contains("ELCODE Forge Dependency Health"));
    assert!(deps_md.contains("`react`"));

    let _ = exit_code;
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Python / FastAPI Fixture E2E Test Suite
// ---------------------------------------------------------------------------

#[test]
fn test_fastapi_app_full_e2e_suite() -> Result<(), Box<dyn Error>> {
    let root = fixture_dir("fastapi-app");

    // 1. Doctor command
    let doctor_report = run_doctor(&root)?;
    assert!(doctor_report.repository_detected);
    let fw = doctor_report
        .checks
        .iter()
        .find(|c| c.label == "Language & framework detection")
        .ok_or("Missing framework check")?;
    assert!(fw.detail.contains("python"));
    assert!(fw.detail.contains("fastapi"));

    let py_hygiene = doctor_report
        .checks
        .iter()
        .find(|c| c.label == "Python manifest & lockfile hygiene")
        .ok_or("Missing Python hygiene check")?;
    assert!(py_hygiene.detail.contains("lockfile present: true"));

    // 2. Scan command
    let scan_opts = ScanOptions {
        repo_root: root.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };
    let (repo, exit_code) = execute_scan(&scan_opts)?;
    let health = repo.health.as_ref().ok_or("Missing health score")?;
    assert!(health.overall > 0);
    assert!(repo
        .files
        .iter()
        .any(|f| f.relative_path.contains("main.py")));
    assert!(repo
        .modules
        .iter()
        .any(|m| m.id.contains("app/services") || m.id.contains("app/routers")));

    let terminal_out = render_repository_report(&repo, OutputFormat::Terminal)?;
    assert!(terminal_out.contains("python"));
    let sarif_out = render_repository_report(&repo, OutputFormat::Sarif)?;
    assert!(sarif_out.contains("\"runs\""));

    // 3. Review command (synthetic diff)
    let synthetic_diff = r#"diff --git a/app/routers/items.py b/app/routers/items.py
--- a/app/routers/items.py
+++ b/app/routers/items.py
@@ -1,3 +1,4 @@
+# modified item route
"#;
    let (review_diff, _) = execute_review(&root, Some(synthetic_diff), false)?;
    assert!(review_diff
        .impact
        .changed_files
        .iter()
        .any(|f| f.path.contains("items.py")));
    let review_json = render_review_report(&review_diff, OutputFormat::Json)?;
    assert!(review_json.contains("\"risk\""));

    // 4. Explain command
    let explain_repo = execute_explain(&root, None)?;
    assert_eq!(explain_repo.target, ".");
    let explain_mod = execute_explain(&root, Some("app/services"))?;
    assert_eq!(explain_mod.target, "app/services");
    assert!(explain_mod
        .files
        .iter()
        .any(|f| f.contains("user_service.py")));

    // 5. Dependencies command
    let deps_json = run_dependencies(&root, OutputFormat::Json)?;
    assert!(deps_json.contains("\"fastapi\""));
    assert!(deps_json.to_lowercase().contains("pypi"));
    let deps_md = run_dependencies(&root, OutputFormat::Markdown)?;
    assert!(deps_md.contains("`fastapi`"));

    let _ = exit_code;
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Go / Gin Microservice Fixture E2E Test Suite
// ---------------------------------------------------------------------------

#[test]
fn test_go_service_full_e2e_suite() -> Result<(), Box<dyn Error>> {
    let root = fixture_dir("go-service");

    // 1. Doctor command
    let doctor_report = run_doctor(&root)?;
    assert!(doctor_report.repository_detected);
    let fw = doctor_report
        .checks
        .iter()
        .find(|c| c.label == "Language & framework detection")
        .ok_or("Missing framework check")?;
    assert!(fw.detail.contains("go"));
    assert!(fw.detail.contains("gin"));

    let go_hygiene = doctor_report
        .checks
        .iter()
        .find(|c| c.label == "Go manifest & lockfile hygiene")
        .ok_or("Missing Go hygiene check")?;
    assert!(go_hygiene.detail.contains("go.sum present: true"));

    // 2. Scan command (detects circular package dependency)
    let scan_opts = ScanOptions {
        repo_root: root.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };
    let (repo, exit_code) = execute_scan(&scan_opts)?;
    let health = repo.health.as_ref().ok_or("Missing health score")?;
    assert!(health.overall > 0);
    assert!(repo
        .files
        .iter()
        .any(|f| f.relative_path.contains("main.go")));
    assert!(repo
        .modules
        .iter()
        .any(|m| m.id.contains("internal/handlers")));
    assert!(repo
        .modules
        .iter()
        .any(|m| m.id.contains("internal/services")));

    // Intentional circular dependency in go-service fixture
    let has_cycle = repo
        .findings
        .iter()
        .any(|f| f.rule_id == "arch/no-circular-dependencies");
    assert!(
        has_cycle,
        "Expected circular dependency finding in go-service"
    );

    let terminal_out = render_repository_report(&repo, OutputFormat::Terminal)?;
    assert!(terminal_out.contains("go"));

    // 3. Review command
    let synthetic_diff = r#"diff --git a/internal/services/user.go b/internal/services/user.go
--- a/internal/services/user.go
+++ b/internal/services/user.go
@@ -1,3 +1,4 @@
+// update service logic
"#;
    let (review_diff, _) = execute_review(&root, Some(synthetic_diff), false)?;
    assert!(review_diff
        .impact
        .changed_files
        .iter()
        .any(|f| f.path.contains("user.go")));

    // 4. Explain command
    let explain_mod = execute_explain(&root, Some("internal/services"))?;
    assert_eq!(explain_mod.target, "internal/services");
    assert!(explain_mod.files.iter().any(|f| f.contains("user.go")));

    // 5. Dependencies command
    let deps_json = run_dependencies(&root, OutputFormat::Json)?;
    assert!(deps_json.contains("github.com/gin-gonic/gin"));
    assert!(deps_json.to_lowercase().contains("go"));
    let deps_md = run_dependencies(&root, OutputFormat::Markdown)?;
    assert!(deps_md.contains("github.com/gin-gonic/gin"));

    let _ = exit_code;
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. Rust / Cargo CLI Fixture E2E Test Suite
// ---------------------------------------------------------------------------

#[test]
fn test_rust_cli_full_e2e_suite() -> Result<(), Box<dyn Error>> {
    let root = fixture_dir("rust-cli");

    // 1. Doctor command
    let doctor_report = run_doctor(&root)?;
    assert!(doctor_report.repository_detected);
    let fw = doctor_report
        .checks
        .iter()
        .find(|c| c.label == "Language & framework detection")
        .ok_or("Missing framework check")?;
    assert!(fw.detail.contains("rust"));

    let cargo_hygiene = doctor_report
        .checks
        .iter()
        .find(|c| c.label == "Rust manifest & lockfile hygiene")
        .ok_or("Missing Rust hygiene check")?;
    assert!(cargo_hygiene.detail.contains("lockfile present: true"));

    // 2. Scan command (detects wildcard dependency)
    let scan_opts = ScanOptions {
        repo_root: root.clone(),
        incremental: false,
        format: OutputFormat::Json,
    };
    let (repo, exit_code) = execute_scan(&scan_opts)?;
    let health = repo.health.as_ref().ok_or("Missing health score")?;
    assert!(health.overall > 0);
    assert!(repo
        .files
        .iter()
        .any(|f| f.relative_path.contains("main.rs")));
    assert!(repo.modules.iter().any(|m| m.id.contains("src/commands")));

    // Intentional wildcard dependency in rust-cli fixture (`log = "*"`)
    let has_wildcard = repo
        .findings
        .iter()
        .any(|f| f.rule_id == "deps/unpinned-version" && f.evidence.contains("log"));
    assert!(
        has_wildcard,
        "Expected unpinned/wildcard dependency finding for log"
    );

    let terminal_out = render_repository_report(&repo, OutputFormat::Terminal)?;
    assert!(terminal_out.contains("rust"));

    // 3. Review command
    let synthetic_diff = r#"diff --git a/src/commands/run.rs b/src/commands/run.rs
--- a/src/commands/run.rs
+++ b/src/commands/run.rs
@@ -1,3 +1,4 @@
+// update command run
"#;
    let (review_diff, _) = execute_review(&root, Some(synthetic_diff), false)?;
    assert!(review_diff
        .impact
        .changed_files
        .iter()
        .any(|f| f.path.contains("run.rs")));

    // 4. Explain command
    let explain_mod = execute_explain(&root, Some("src/commands"))?;
    assert_eq!(explain_mod.target, "src/commands");
    assert!(explain_mod.files.iter().any(|f| f.contains("run.rs")));

    // 5. Dependencies command
    let deps_json = run_dependencies(&root, OutputFormat::Json)?;
    assert!(deps_json.contains("\"log\""));
    assert!(deps_json.to_lowercase().contains("cargo"));
    let deps_md = run_dependencies(&root, OutputFormat::Markdown)?;
    assert!(deps_md.contains("`log`"));

    let _ = exit_code;
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. Polyglot Cross-Language Parity & Determinism Verification
// ---------------------------------------------------------------------------

#[test]
fn test_polyglot_cross_language_parity_and_determinism() -> Result<(), Box<dyn Error>> {
    let fixtures = ["next-app", "fastapi-app", "go-service", "rust-cli"];

    for fixture_name in fixtures {
        let root = fixture_dir(fixture_name);

        // Scan run 1
        let scan_opts_1 = ScanOptions {
            repo_root: root.clone(),
            incremental: false,
            format: OutputFormat::Json,
        };
        let (repo_1, _) = execute_scan(&scan_opts_1)?;
        let json_1 = serde_json::to_string_pretty(&repo_1)?;

        // Scan run 2
        let scan_opts_2 = ScanOptions {
            repo_root: root.clone(),
            incremental: false,
            format: OutputFormat::Json,
        };
        let (repo_2, _) = execute_scan(&scan_opts_2)?;
        let json_2 = serde_json::to_string_pretty(&repo_2)?;

        // Parity & determinism assertions
        assert_eq!(
            json_1, json_2,
            "Non-deterministic scan output detected for fixture '{fixture_name}'"
        );

        let h1 = repo_1.health.as_ref().ok_or("Missing health score")?;
        let h2 = repo_2.health.as_ref().ok_or("Missing health score")?;

        assert_eq!(h1.overall, h2.overall);
        assert_eq!(h1.dimensions.len(), 8);

        // Verify total dimension weight sums to 100%
        let total_weight: u8 = h1.dimensions.iter().map(|d| d.weight_pct).sum();
        assert_eq!(
            total_weight, 100,
            "Health dimension weights for '{fixture_name}' must sum to 100%"
        );
    }

    Ok(())
}
