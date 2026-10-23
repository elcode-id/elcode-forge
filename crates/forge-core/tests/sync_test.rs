use forge_core::sync::{
    compute_sync_content_hash, load_credentials_from_path, resolve_token, save_credentials_to_path,
    CredentialsConfig, DeltaSummary, ServerCredential, SyncManifest, SyncPullResponse,
    SyncPushPayload, SyncStatus,
};
use forge_core::{Confidence, FileNode, Finding, HealthScore, Severity, SourceLocation};
use std::collections::BTreeMap;
use std::fs;

fn make_finding(id: &str, rule: &str, sev: Severity) -> Finding {
    Finding {
        id: id.to_string(),
        category: "maintainability".to_string(),
        severity: sev,
        confidence: Confidence::Certain,
        title: format!("Finding {id}"),
        description: format!("Description for {id}"),
        evidence: "sample evidence line".to_string(),
        location: Some(SourceLocation {
            file_path: "src/lib.rs".to_string(),
            start_line: Some(15),
            end_line: Some(25),
        }),
        recommendation: "Refactor function".to_string(),
        rule_id: rule.to_string(),
    }
}

fn make_file(path: &str, lines: u32, hash: &str) -> FileNode {
    FileNode {
        relative_path: path.to_string(),
        language: Some("rust".to_string()),
        size_bytes: lines as u64 * 30,
        line_count: lines,
        content_hash: hash.to_string(),
        is_test: false,
    }
}

#[test]
fn test_sync_protocol_complete_workflow() {
    let manifest = SyncManifest {
        repo_id: "elcode-id/elcode-forge".to_string(),
        repo_name: "elcode-forge".to_string(),
        branch: "develop".to_string(),
        commit_hash: "9f8e7d6c5b4a".to_string(),
        timestamp_secs: 1775000000,
        content_hash: "blake3_hash_abcdef1234567890".to_string(),
        tool_version: "0.9.0".to_string(),
    };

    let f1 = make_finding("F1", "rule-01", Severity::Low);
    let f2 = make_finding("F2", "rule-02", Severity::High);

    let push = SyncPushPayload {
        manifest: manifest.clone(),
        findings_count: 2,
        health_score: HealthScore {
            overall: 88,
            dimensions: vec![],
        },
        findings: vec![f1.clone(), f2.clone()],
        modules: vec![],
        metadata: BTreeMap::from([
            ("ci_provider".to_string(), "github-actions".to_string()),
            ("run_id".to_string(), "12345678".to_string()),
        ]),
    };

    // Serialize push payload
    let json_push = serde_json::to_string_pretty(&push);
    assert!(json_push.is_ok());
    if let Ok(json_str) = json_push {
        let deserialized_push: Result<SyncPushPayload, _> = serde_json::from_str(&json_str);
        assert!(deserialized_push.is_ok());
        if let Ok(d) = deserialized_push {
            assert_eq!(d.findings_count, 2);
            assert_eq!(d.manifest.repo_id, "elcode-id/elcode-forge");
        }
    }

    // Pull response test
    let pull = SyncPullResponse {
        repo_id: "elcode-id/elcode-forge".to_string(),
        branch: "develop".to_string(),
        baseline_commit_hash: Some("baseline_sha_000".to_string()),
        baseline_health_score: Some(HealthScore {
            overall: 90,
            dimensions: vec![],
        }),
        baseline_findings: vec![f1.clone()],
        baseline_content_hash: Some("baseline_content_hash_111".to_string()),
        sync_status: SyncStatus::FastForward,
    };

    let json_pull = serde_json::to_string(&pull);
    assert!(json_pull.is_ok());
    if let Ok(json_str) = json_pull {
        let deserialized_pull: Result<SyncPullResponse, _> = serde_json::from_str(&json_str);
        assert!(deserialized_pull.is_ok());
        if let Ok(d) = deserialized_pull {
            assert_eq!(d.sync_status, SyncStatus::FastForward);
            assert_eq!(d.baseline_findings.len(), 1);
        }
    }

    // Delta summary computation
    let delta = DeltaSummary::compute(
        &push.findings,
        &pull.baseline_findings,
        &push.health_score,
        pull.baseline_health_score.as_ref(),
    );

    assert_eq!(delta.added_findings, 1); // F2 was added
    assert_eq!(delta.resolved_findings, 0);
    assert_eq!(delta.unchanged_findings, 1); // F1 is in both
    assert_eq!(delta.health_score_delta, -2); // 88 - 90 = -2
    assert!(!delta.is_clean);
}

#[test]
fn test_delta_hashing_and_canonical_sorting() {
    let file1 = make_file("src/analyzer.rs", 150, "hash_file1");
    let file2 = make_file("src/config.rs", 250, "hash_file2");
    let file3 = make_file("src/main.rs", 80, "hash_file3");

    let finding1 = make_finding("finding-001", "rule-a", Severity::Medium);
    let finding2 = make_finding("finding-002", "rule-b", Severity::Critical);

    // Compute hash with one ordering
    let hash1 = compute_sync_content_hash(
        &[file1.clone(), file2.clone(), file3.clone()],
        &[finding1.clone(), finding2.clone()],
    );

    // Compute hash with reverse ordering
    let hash2 = compute_sync_content_hash(
        &[file3.clone(), file1.clone(), file2.clone()],
        &[finding2.clone(), finding1.clone()],
    );

    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64);
}

#[test]
fn test_credential_storage_and_token_resolution() {
    let temp_dir = std::env::temp_dir().join(format!("eforge_sync_test_{}", std::process::id()));
    let cred_path = temp_dir.join("credentials.yaml");

    let mut creds = CredentialsConfig::new();
    creds.default_server = Some("https://forge.elcode.id".to_string());
    creds.set_credential(ServerCredential {
        server_url: "https://forge.elcode.id".to_string(),
        token: "live_token_777".to_string(),
        username: Some("elcode-bot".to_string()),
        expires_at: Some(2000000000),
    });
    creds.set_credential(ServerCredential {
        server_url: "https://internal.forge.corp".to_string(),
        token: "corp_token_888".to_string(),
        username: Some("corp-user".to_string()),
        expires_at: None,
    });

    let save_res = save_credentials_to_path(&creds, &cred_path);
    assert!(save_res.is_ok());

    let loaded_res = load_credentials_from_path(&cred_path);
    assert!(loaded_res.is_ok());
    if let Ok(loaded) = loaded_res {
        assert_eq!(loaded.servers.len(), 2);
        assert_eq!(
            loaded.resolve_token_for(Some("https://internal.forge.corp")),
            Some("corp_token_888".to_string())
        );
        assert_eq!(
            loaded.resolve_token_for(None),
            Some("live_token_777".to_string())
        );
    }

    // Test environment variable token resolution
    std::env::set_var("EFORGE_API_TOKEN", "env_override_token_999");
    let resolved = resolve_token(Some("https://forge.elcode.id"));
    assert_eq!(resolved, Some("env_override_token_999".to_string()));
    std::env::remove_var("EFORGE_API_TOKEN");

    let _ = fs::remove_dir_all(&temp_dir);
}
