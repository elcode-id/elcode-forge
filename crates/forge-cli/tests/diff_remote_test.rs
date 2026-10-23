use std::fs;
use std::path::PathBuf;

use forge_cli::commands::diff::run_diff_remote;
use forge_cli::commands::sync::save_cached_baseline;
use forge_cli::exit_code::ExitCode;
use forge_cli::OutputFormat;
use forge_core::ir::{Confidence, Finding, HealthScore, Severity, SourceLocation};
use forge_core::sync::{SyncPullResponse, SyncStatus};

fn create_temp_repo(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "eforge_diff_remote_{}_{}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::create_dir_all(dir.join("src"));

    let config = "version: 1\nproject_name: test-diff-repo\nlanguages: [\"rust\"]\n";
    let _ = fs::write(dir.join("eforge.config.yaml"), config);
    let _ = fs::write(
        dir.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
    );

    dir
}

fn sample_finding(id: &str, title: &str) -> Finding {
    Finding {
        id: id.to_string(),
        category: "maintainability".to_string(),
        severity: Severity::Medium,
        confidence: Confidence::High,
        title: title.to_string(),
        description: "Test finding description".to_string(),
        evidence: "pub fn add".to_string(),
        location: Some(SourceLocation {
            file_path: "src/lib.rs".to_string(),
            start_line: Some(1),
            end_line: Some(1),
        }),
        recommendation: "Refactor if necessary".to_string(),
        rule_id: "test-rule".to_string(),
    }
}

fn sample_health_score(overall: u8) -> HealthScore {
    HealthScore {
        overall,
        dimensions: Vec::new(),
    }
}

#[test]
fn test_diff_remote_missing_baseline_returns_config_error() {
    let repo = create_temp_repo("missing_base");

    let code_term = run_diff_remote(&repo, OutputFormat::Terminal);
    assert_eq!(code_term, ExitCode::ConfigurationError);

    let code_json = run_diff_remote(&repo, OutputFormat::Json);
    assert_eq!(code_json, ExitCode::ConfigurationError);

    let _ = fs::remove_dir_all(&repo);
}

#[test]
fn test_diff_remote_with_cached_baseline_terminal_and_json() {
    let repo = create_temp_repo("with_base");

    let baseline = SyncPullResponse {
        repo_id: "elcode-id/test-diff-repo".to_string(),
        branch: "develop".to_string(),
        baseline_commit_hash: Some("abcdef1234567890".to_string()),
        baseline_health_score: Some(sample_health_score(90)),
        baseline_findings: vec![sample_finding("baseline_f1", "Legacy issue")],
        baseline_content_hash: Some("mock_content_hash".to_string()),
        sync_status: SyncStatus::FastForward,
    };

    let save_res = save_cached_baseline(&repo, &baseline);
    assert!(save_res.is_ok());

    let exit_term = run_diff_remote(&repo, OutputFormat::Terminal);
    assert_eq!(exit_term, ExitCode::Success);

    let exit_json = run_diff_remote(&repo, OutputFormat::Json);
    assert_eq!(exit_json, ExitCode::Success);

    let _ = fs::remove_dir_all(&repo);
}

#[test]
fn test_diff_remote_detects_clean_baseline() {
    let repo = create_temp_repo("clean_base");

    let baseline = SyncPullResponse {
        repo_id: "elcode-id/test-diff-repo".to_string(),
        branch: "main".to_string(),
        baseline_commit_hash: Some("1122334455667788".to_string()),
        baseline_health_score: Some(sample_health_score(100)),
        baseline_findings: Vec::new(),
        baseline_content_hash: Some("hash100".to_string()),
        sync_status: SyncStatus::UpToDate,
    };

    let save_res = save_cached_baseline(&repo, &baseline);
    assert!(save_res.is_ok());

    let exit = run_diff_remote(&repo, OutputFormat::Json);
    assert_eq!(exit, ExitCode::Success);

    let _ = fs::remove_dir_all(&repo);
}
