//! End-to-end integration tests for git hooks lifecycle, status, and staged diff scanning.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use forge_cli::cli::OutputFormat;
use forge_cli::commands::hook::{
    render_hook_run, render_hook_status, run_hook_install, run_hook_run, run_hook_status,
    run_hook_uninstall,
};
use forge_cli::exit_code::ExitCode;
use forge_core::ForgeError;

/// Helper to initialize a clean temporary Git repository for testing.
struct TempGitRepo {
    path: PathBuf,
}

impl TempGitRepo {
    fn new(prefix: &str) -> Result<Self, ForgeError> {
        let nanos = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_nanos(),
            Err(_) => 0,
        };
        let temp_dir = std::env::temp_dir().join(format!("eforge-test-{prefix}-{nanos}"));
        fs::create_dir_all(&temp_dir).map_err(|e| ForgeError::Io {
            path: temp_dir.clone(),
            source: e,
        })?;

        // Initialize git repository
        let init_status = Command::new("git")
            .arg("init")
            .current_dir(&temp_dir)
            .status()
            .map_err(|e| ForgeError::Io {
                path: temp_dir.clone(),
                source: e,
            })?;

        if !init_status.success() {
            return Err(ForgeError::Analysis {
                analyzer: "git",
                message: "Failed to initialize git test repository".to_string(),
            });
        }

        // Configure dummy user identity for git
        let _ = Command::new("git")
            .args(["config", "user.name", "ELCODE Forge Test"])
            .current_dir(&temp_dir)
            .status();

        let _ = Command::new("git")
            .args(["config", "user.email", "test@elcode.id"])
            .current_dir(&temp_dir)
            .status();

        Ok(Self { path: temp_dir })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempGitRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_hooks_lifecycle_install_status_and_uninstall() -> Result<(), ForgeError> {
    let repo = TempGitRepo::new("lifecycle")?;
    let repo_root = repo.path();

    // 1. Initial status: neither hook installed
    let status_init = run_hook_status(repo_root)?;
    let status_init_json = render_hook_status(&status_init, OutputFormat::Json)?;
    assert!(status_init_json.contains("\"installed\": false"));

    // 2. Install all hooks (pre-commit and pre-push)
    let install_res = run_hook_install(repo_root, "all", false)?;
    assert_eq!(install_res.installed.len(), 2);
    assert_eq!(install_res.skipped.len(), 0);

    let pre_commit_path = repo_root.join(".git").join("hooks").join("pre-commit");
    let pre_push_path = repo_root.join(".git").join("hooks").join("pre-push");
    assert!(pre_commit_path.is_file());
    assert!(pre_push_path.is_file());

    let pre_commit_content = fs::read_to_string(&pre_commit_path).map_err(|e| ForgeError::Io {
        path: pre_commit_path.clone(),
        source: e,
    })?;
    assert!(pre_commit_content.contains("BEGIN ELCODE FORGE HOOK"));
    assert!(pre_commit_content.contains("FORGE_SKIP_HOOK"));

    // 3. Status after install: both hooks installed and managed by Forge
    let status_installed = run_hook_status(repo_root)?;
    let status_inst_json = render_hook_status(&status_installed, OutputFormat::Json)?;
    assert!(status_inst_json.contains("\"installed\": true"));
    assert!(status_inst_json.contains("\"managed_by_forge\": true"));

    // 4. Re-install without force should skip
    let install_skip = run_hook_install(repo_root, "pre-commit", false)?;
    assert_eq!(install_skip.skipped.len(), 1);
    assert_eq!(install_skip.installed.len(), 0);

    // 5. Re-install with force should overwrite cleanly
    let install_force = run_hook_install(repo_root, "pre-commit", true)?;
    assert_eq!(install_force.installed.len(), 1);

    // 6. Uninstall hooks
    let uninstall_res = run_hook_uninstall(repo_root, "all")?;
    assert_eq!(uninstall_res.uninstalled.len(), 2);
    assert!(!pre_commit_path.exists());
    assert!(!pre_push_path.exists());

    // 7. Status after uninstall: not installed
    let status_final = run_hook_status(repo_root)?;
    let status_final_json = render_hook_status(&status_final, OutputFormat::Json)?;
    assert!(status_final_json.contains("\"installed\": false"));

    Ok(())
}

#[test]
fn test_hook_run_pre_commit_clean_when_no_staged_files() -> Result<(), ForgeError> {
    let repo = TempGitRepo::new("staged-clean")?;
    let repo_root = repo.path();

    let (report, exit_code) = run_hook_run(repo_root, "pre-commit", None)?;
    assert_eq!(exit_code, ExitCode::Success);
    assert!(report.staged_files.is_empty());

    let rendered = render_hook_run(&report, OutputFormat::Terminal)?;
    assert!(rendered.contains("No staged code files to check. Hook passed."));

    Ok(())
}

#[test]
fn test_hook_run_pre_commit_detects_violations_in_staged_changes() -> Result<(), ForgeError> {
    let repo = TempGitRepo::new("staged-violation")?;
    let repo_root = repo.path();

    // 1. Create a minimal Next.js / TypeScript structure
    let pkg_json = r#"{
  "name": "hooks-test-app",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.2.0"
  }
}"#;
    fs::write(repo_root.join("package.json"), pkg_json).map_err(|e| ForgeError::Io {
        path: repo_root.join("package.json"),
        source: e,
    })?;

    let db_dir = repo_root.join("src").join("db");
    fs::create_dir_all(&db_dir).map_err(|e| ForgeError::Io {
        path: db_dir.clone(),
        source: e,
    })?;
    fs::write(
        db_dir.join("client.ts"),
        "export const db = { query: () => [] };",
    )
    .map_err(|e| ForgeError::Io {
        path: db_dir.join("client.ts"),
        source: e,
    })?;

    let ui_dir = repo_root.join("src").join("components");
    fs::create_dir_all(&ui_dir).map_err(|e| ForgeError::Io {
        path: ui_dir.clone(),
        source: e,
    })?;

    // Direct DB import from UI component violates arch/no-direct-db-in-ui
    let ui_content = "import { db } from '../db/client';\nexport const Card = () => db.query();";
    fs::write(ui_dir.join("Card.tsx"), ui_content).map_err(|e| ForgeError::Io {
        path: ui_dir.join("Card.tsx"),
        source: e,
    })?;

    // 2. Stage files in git
    let add_status = Command::new("git")
        .args([
            "add",
            "package.json",
            "src/db/client.ts",
            "src/components/Card.tsx",
        ])
        .current_dir(repo_root)
        .status()
        .map_err(|e| ForgeError::Io {
            path: repo_root.to_path_buf(),
            source: e,
        })?;
    assert!(add_status.success());

    // 3. Run hook check on staged changes
    let (report, exit_code) = run_hook_run(repo_root, "pre-commit", None)?;

    // Hook should block commit with non-zero exit code
    assert_ne!(exit_code, ExitCode::Success);
    assert!(!report.findings.is_empty());

    let rendered = render_hook_run(&report, OutputFormat::Terminal)?;
    assert!(rendered.contains("FAILED"));
    assert!(rendered.contains("arch/no-direct-db-in-ui"));

    Ok(())
}

#[test]
fn test_hook_status_formatting_and_pre_push_clean() -> Result<(), ForgeError> {
    let repo = TempGitRepo::new("status-formats")?;
    let repo_root = repo.path();

    // 1. Install hooks
    let _ = run_hook_install(repo_root, "all", false)?;

    // 2. Format status in terminal and markdown
    let status = run_hook_status(repo_root)?;
    let term_out = render_hook_status(&status, OutputFormat::Terminal)?;
    assert!(term_out.contains("ELCODE FORGE — Git Hooks Status"));
    assert!(term_out.contains("pre-commit"));
    assert!(term_out.contains("pre-push"));

    let md_out = render_hook_status(&status, OutputFormat::Markdown)?;
    assert!(md_out.contains("# Git Hooks Status"));
    assert!(md_out.contains("| Hook | Installed | Managed by Forge | Has Backup |"));

    // 3. Test pre-push run clean
    let (push_report, push_exit) = run_hook_run(repo_root, "pre-push", None)?;
    assert_eq!(push_exit, ExitCode::Success);
    assert_eq!(push_report.hook_name, "pre-push");

    Ok(())
}
