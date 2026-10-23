//! Integration tests for `forge-server` endpoints and synchronization workflow.

use std::collections::BTreeMap;
use std::error::Error;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use forge_core::ir::{Confidence, Finding, HealthScore, Severity, SourceLocation};
use forge_core::sync::{SyncManifest, SyncPullResponse, SyncPushPayload, SyncStatus};
use forge_server::{create_app, hash_api_key, issue_jwt, AppState, Db, ServerConfig};

async fn setup_test_app() -> Result<(axum::Router, AppState), Box<dyn Error>> {
    let db = Db::connect_in_memory().await?;
    let config = ServerConfig {
        admin_api_key: Some("test-admin-secret-key-12345".to_string()),
        jwt_secret: "test-jwt-secret-string-at-least-32-characters-long".to_string(),
        ..Default::default()
    };

    let now = chrono::Utc::now().timestamp();
    let client_key_hash = hash_api_key("test-client-api-key");
    db.create_api_key("k1", "client-app", &client_key_hash, now, None)
        .await?;

    let state = AppState::new(db, config);
    let app = create_app(state.clone());
    Ok((app, state))
}

fn sample_push_payload(repo_id: &str, branch: &str, commit: &str) -> SyncPushPayload {
    let finding = Finding {
        id: "F-001".to_string(),
        category: "maintainability".to_string(),
        severity: Severity::Medium,
        confidence: Confidence::High,
        title: "Test finding title".to_string(),
        description: "Test description".to_string(),
        evidence: "let x = 1;".to_string(),
        location: Some(SourceLocation {
            file_path: "src/main.rs".to_string(),
            start_line: Some(10),
            end_line: Some(12),
        }),
        recommendation: "Refactor if necessary".to_string(),
        rule_id: "test-rule".to_string(),
    };

    let manifest = SyncManifest {
        repo_id: repo_id.to_string(),
        repo_name: "test-project".to_string(),
        branch: branch.to_string(),
        commit_hash: commit.to_string(),
        timestamp_secs: 1_700_000_000,
        content_hash: "blake3_mock_content_hash_123456789".to_string(),
        tool_version: "0.9.0".to_string(),
    };

    SyncPushPayload {
        manifest,
        findings_count: 1,
        health_score: HealthScore {
            overall: 95,
            dimensions: Vec::new(),
        },
        findings: vec![finding],
        modules: Vec::new(),
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
async fn test_health_endpoint() -> Result<(), Box<dyn Error>> {
    let (app, _) = setup_test_app().await?;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/health")
        .body(Body::empty())?;

    let res = app.oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = to_bytes(res.into_body(), 1024 * 64).await?;
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;

    assert_eq!(json["status"], "ok");
    assert_eq!(json["database"], "sqlite");
    assert!(!json["version"].as_str().unwrap_or_default().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_auth_token_issuance_and_rejection() -> Result<(), Box<dyn Error>> {
    let (app, _) = setup_test_app().await?;

    // 1. Valid API key issuance
    let valid_body = serde_json::json!({
        "api_key": "test-client-api-key",
        "username": "tester"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/token")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&valid_body)?))?;

    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = to_bytes(res.into_body(), 1024 * 64).await?;
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert!(json["access_token"].is_string());
    assert_eq!(json["token_type"], "Bearer");

    // 2. Invalid API key rejected with 401
    let invalid_body = serde_json::json!({
        "api_key": "wrong-secret-key"
    });
    let req_invalid = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/token")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&invalid_body)?))?;

    let res_invalid = app.oneshot(req_invalid).await?;
    assert_eq!(res_invalid.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

#[tokio::test]
async fn test_protected_endpoints_require_authorization() -> Result<(), Box<dyn Error>> {
    let (app, _) = setup_test_app().await?;

    // Missing auth header
    let req_pull = Request::builder()
        .method("GET")
        .uri("/api/v1/sync/pull?repo_id=test/repo&branch=main")
        .body(Body::empty())?;
    let res_pull = app.clone().oneshot(req_pull).await?;
    assert_eq!(res_pull.status(), StatusCode::UNAUTHORIZED);

    // Invalid token format
    let req_invalid = Request::builder()
        .method("GET")
        .uri("/api/v1/sync/pull?repo_id=test/repo&branch=main")
        .header("Authorization", "Bearer invalid-garbage-token")
        .body(Body::empty())?;
    let res_invalid = app.oneshot(req_invalid).await?;
    assert_eq!(res_invalid.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

#[tokio::test]
async fn test_sync_push_and_pull_lifecycle() -> Result<(), Box<dyn Error>> {
    let (app, state) = setup_test_app().await?;

    // Issue JWT for client
    let token = issue_jwt(&state.config.jwt_secret, "ci-runner", "agent", 3600)?;

    // 1. Initial Pull before any push returns NotFound
    let req_pull_initial = Request::builder()
        .method("GET")
        .uri("/api/v1/sync/pull?repo_id=elcode-id/demo&branch=develop")
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())?;

    let res_pull_initial = app.clone().oneshot(req_pull_initial).await?;
    assert_eq!(res_pull_initial.status(), StatusCode::OK);

    let bytes = to_bytes(res_pull_initial.into_body(), 1024 * 64).await?;
    let initial_resp: SyncPullResponse = serde_json::from_slice(&bytes)?;
    assert_eq!(initial_resp.sync_status, SyncStatus::NotFound);
    assert!(initial_resp.baseline_commit_hash.is_none());

    // 2. Push snapshot
    let payload = sample_push_payload("elcode-id/demo", "develop", "commit_abc123");
    let req_push = Request::builder()
        .method("POST")
        .uri("/api/v1/sync/push")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload)?))?;

    let res_push = app.clone().oneshot(req_push).await?;
    assert_eq!(res_push.status(), StatusCode::OK);

    let push_bytes = to_bytes(res_push.into_body(), 1024 * 64).await?;
    let push_resp: SyncPullResponse = serde_json::from_slice(&push_bytes)?;
    assert_eq!(push_resp.sync_status, SyncStatus::BaselineUpdated);
    assert_eq!(
        push_resp.baseline_commit_hash.as_deref(),
        Some("commit_abc123")
    );
    assert_eq!(push_resp.baseline_findings.len(), 1);

    // 3. Pull retrieves newly pushed snapshot
    let req_pull = Request::builder()
        .method("GET")
        .uri("/api/v1/sync/pull?repo_id=elcode-id/demo&branch=develop")
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())?;

    let res_pull = app.oneshot(req_pull).await?;
    assert_eq!(res_pull.status(), StatusCode::OK);

    let pull_bytes = to_bytes(res_pull.into_body(), 1024 * 64).await?;
    let pull_resp: SyncPullResponse = serde_json::from_slice(&pull_bytes)?;
    assert_eq!(pull_resp.sync_status, SyncStatus::UpToDate);
    assert_eq!(
        pull_resp.baseline_commit_hash.as_deref(),
        Some("commit_abc123")
    );
    assert_eq!(pull_resp.baseline_findings.len(), 1);
    assert_eq!(pull_resp.baseline_findings[0].id, "F-001");
    if let Some(health) = pull_resp.baseline_health_score {
        assert_eq!(health.overall, 95);
    } else {
        return Err("Missing baseline health score".into());
    }

    Ok(())
}

#[tokio::test]
async fn test_forge_server_tcp_http_client_roundtrip() -> Result<(), Box<dyn Error>> {
    let (app, _) = setup_test_app().await?;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;
    let base_url = format!("http://{}", local_addr);

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let client = reqwest::Client::new();

    // 1. Health check over real TCP socket
    let health_resp = client
        .get(format!("{base_url}/api/v1/health"))
        .send()
        .await?;
    assert_eq!(health_resp.status(), reqwest::StatusCode::OK);
    let health_json: serde_json::Value = health_resp.json().await?;
    assert_eq!(health_json["status"], "ok");

    // 2. Exchange API key for session token
    let auth_resp = client
        .post(format!("{base_url}/api/v1/auth/token"))
        .json(&serde_json::json!({
            "api_key": "test-client-api-key",
            "username": "client-app"
        }))
        .send()
        .await?;
    assert_eq!(auth_resp.status(), reqwest::StatusCode::OK);
    let auth_json: serde_json::Value = auth_resp.json().await?;
    let token = match auth_json["access_token"].as_str() {
        Some(t) => t.to_string(),
        None => return Err("Missing access token".into()),
    };

    // 3. Push payload over HTTP
    let payload = sample_push_payload("elcode-id/tcp-demo", "main", "sha_112233");
    let push_resp = client
        .post(format!("{base_url}/api/v1/sync/push"))
        .bearer_auth(&token)
        .json(&payload)
        .send()
        .await?;
    assert_eq!(push_resp.status(), reqwest::StatusCode::OK);
    let push_data: SyncPullResponse = push_resp.json().await?;
    assert_eq!(push_data.sync_status, SyncStatus::BaselineUpdated);
    assert_eq!(
        push_data.baseline_commit_hash.as_deref(),
        Some("sha_112233")
    );

    // 4. Pull payload over HTTP
    let pull_resp = client
        .get(format!("{base_url}/api/v1/sync/pull"))
        .query(&[("repo_id", "elcode-id/tcp-demo"), ("branch", "main")])
        .bearer_auth(&token)
        .send()
        .await?;
    assert_eq!(pull_resp.status(), reqwest::StatusCode::OK);
    let pull_data: SyncPullResponse = pull_resp.json().await?;
    assert_eq!(pull_data.sync_status, SyncStatus::UpToDate);
    assert_eq!(
        pull_data.baseline_commit_hash.as_deref(),
        Some("sha_112233")
    );
    assert_eq!(pull_data.baseline_findings.len(), 1);
    assert_eq!(pull_data.baseline_findings[0].id, "F-001");

    Ok(())
}
