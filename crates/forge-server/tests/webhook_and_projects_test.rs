//! Integration tests for workspace registry, multi-branch history retention, and outbound webhooks.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use forge_core::ir::{Confidence, DimensionScore, Finding, HealthScore, Severity, SourceLocation};
use forge_core::sync::{SyncManifest, SyncPushPayload};
use forge_server::{
    create_app, format_discord_payload, format_slack_payload, generate_hmac_signature,
    verify_hmac_signature, AppState, Db, ServerConfig, WebhookEvent, WebhookPayload,
};

async fn setup_test_app() -> Result<(axum::Router, AppState), Box<dyn Error>> {
    let db = Db::connect_in_memory().await?;
    let config = ServerConfig {
        admin_api_key: Some("test-admin-secret-key-12345".to_string()),
        jwt_secret: "test-jwt-secret-string-at-least-32-characters-long".to_string(),
        ..Default::default()
    };

    let state = AppState::new(db, config);
    let app = create_app(state.clone());
    Ok((app, state))
}

fn sample_push(
    repo_id: &str,
    repo_name: &str,
    branch: &str,
    commit: &str,
    overall_health: u8,
    findings: Vec<Finding>,
) -> SyncPushPayload {
    let manifest = SyncManifest {
        repo_id: repo_id.to_string(),
        repo_name: repo_name.to_string(),
        branch: branch.to_string(),
        commit_hash: commit.to_string(),
        timestamp_secs: chrono::Utc::now().timestamp().max(0) as u64,
        content_hash: format!("hash_{commit}"),
        tool_version: "0.9.0".to_string(),
    };

    let health_score = HealthScore {
        overall: overall_health,
        dimensions: vec![
            DimensionScore {
                dimension: "architecture".to_string(),
                score: overall_health,
                weight_pct: 35,
                reasons: Vec::new(),
            },
            DimensionScore {
                dimension: "security".to_string(),
                score: overall_health,
                weight_pct: 35,
                reasons: Vec::new(),
            },
            DimensionScore {
                dimension: "maintainability".to_string(),
                score: overall_health,
                weight_pct: 30,
                reasons: Vec::new(),
            },
        ],
    };

    SyncPushPayload {
        manifest,
        findings_count: findings.len(),
        health_score,
        findings,
        modules: Vec::new(),
        metadata: BTreeMap::new(),
    }
}

fn sample_critical_finding(id: &str) -> Finding {
    Finding {
        id: id.to_string(),
        category: "security".to_string(),
        severity: Severity::Critical,
        confidence: Confidence::Certain,
        title: "Hardcoded API secret token exposed".to_string(),
        description: "Found live production credentials committed in source code".to_string(),
        evidence: "const SECRET = 'sk_live_123456789';".to_string(),
        location: Some(SourceLocation {
            file_path: "src/config.ts".to_string(),
            start_line: Some(15),
            end_line: Some(15),
        }),
        recommendation: "Use environment variables and rotate secret immediately".to_string(),
        rule_id: "sec/no-hardcoded-secret".to_string(),
    }
}

#[tokio::test]
async fn test_project_registry_and_branches() -> Result<(), Box<dyn Error>> {
    let (app, _) = setup_test_app().await?;
    let admin_auth = "Bearer test-admin-secret-key-12345";

    // 1. Push initial snapshot to main
    let push1 = sample_push(
        "elcode-core",
        "ELCODE Core",
        "main",
        "c0000001",
        90,
        Vec::new(),
    );
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sync/push")
        .header("Authorization", admin_auth)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&push1)?))?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Push another branch: feature/auth
    let push2 = sample_push(
        "elcode-core",
        "ELCODE Core",
        "feature/auth",
        "c0000002",
        85,
        vec![sample_critical_finding("F-SEC-1")],
    );
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sync/push")
        .header("Authorization", admin_auth)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&push2)?))?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::OK);

    // 3. Query projects list: GET /api/v1/projects
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects")
        .header("Authorization", admin_auth)
        .body(Body::empty())?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), 1024 * 64).await?;
    let projects: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert!(projects.is_array());
    assert_eq!(projects.as_array().map(|a| a.len()), Some(1));
    assert_eq!(projects[0]["repo_id"], "elcode-core");
    assert_eq!(projects[0]["name"], "ELCODE Core");

    // 4. Query project overview: GET /api/v1/projects/elcode-core/overview
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/elcode-core/overview")
        .header("Authorization", admin_auth)
        .body(Body::empty())?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), 1024 * 64).await?;
    let overview: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(overview["repo_id"], "elcode-core");
    assert_eq!(overview["branches_count"], 2);
    assert_eq!(overview["latest_health"]["overall"], 90);

    // 5. Query branches: GET /api/v1/projects/elcode-core/branches
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/elcode-core/branches")
        .header("Authorization", admin_auth)
        .body(Body::empty())?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), 1024 * 64).await?;
    let branches: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(branches.as_array().map(|a| a.len()), Some(2));

    // 6. Test 404 for non-existent project overview
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/non-existent-repo/overview")
        .header("Authorization", admin_auth)
        .body(Body::empty())?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn test_multi_branch_history_retention_and_trends() -> Result<(), Box<dyn Error>> {
    let (app, _) = setup_test_app().await?;
    let admin_auth = "Bearer test-admin-secret-key-12345";

    // Push 3 snapshots to main with improving scores
    for (i, score) in [70, 80, 92].iter().enumerate() {
        tokio::time::sleep(Duration::from_millis(10)).await;
        let push = sample_push(
            "trend-repo",
            "Trend Project",
            "main",
            &format!("commit_{i}"),
            *score,
            Vec::new(),
        );
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/sync/push")
            .header("Authorization", admin_auth)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&push)?))?;
        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::OK);
    }

    // Query history endpoint: GET /api/v1/projects/trend-repo/history?branch=main
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/trend-repo/history?branch=main&limit=10")
        .header("Authorization", admin_auth)
        .body(Body::empty())?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), 1024 * 64).await?;
    let trend: serde_json::Value = serde_json::from_slice(&bytes)?;

    assert_eq!(trend["repo_id"], "trend-repo");
    assert_eq!(trend["branch"], "main");
    assert_eq!(trend["total_points"], 3);
    assert_eq!(trend["latest_health"], 92);
    assert_eq!(trend["trend_direction"], "improving");
    assert_eq!(trend["health_delta"], 22);
    assert_eq!(trend["points"].as_array().map(|a| a.len()), Some(3));

    Ok(())
}

#[tokio::test]
async fn test_webhook_crud_lifecycle() -> Result<(), Box<dyn Error>> {
    let (app, _) = setup_test_app().await?;
    let admin_auth = "Bearer test-admin-secret-key-12345";

    // 1. Create a webhook subscription: POST /api/v1/webhooks
    let webhook_body = serde_json::json!({
        "repo_id": "test-repo",
        "url": "https://example.com/webhook",
        "secret": "my-secret-key-123",
        "events": ["critical_vulnerability", "health_score_drop"],
        "min_health_threshold": 80
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/webhooks")
        .header("Authorization", admin_auth)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&webhook_body)?))?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::CREATED);

    let bytes = to_bytes(res.into_body(), 1024 * 64).await?;
    let created: serde_json::Value = serde_json::from_slice(&bytes)?;
    let webhook_id = created["id"].as_str().unwrap_or_default().to_string();
    assert!(webhook_id.starts_with("wh_"));
    assert_eq!(created["repo_id"], "test-repo");
    assert_eq!(created["has_secret"], true);
    assert_eq!(created["is_active"], true);

    // 2. List webhooks: GET /api/v1/webhooks
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/webhooks?repo_id=test-repo")
        .header("Authorization", admin_auth)
        .body(Body::empty())?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = to_bytes(res.into_body(), 1024 * 64).await?;
    let list: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(list.as_array().map(|a| a.len()), Some(1));
    assert_eq!(list[0]["id"], webhook_id);

    // 3. Delete webhook: DELETE /api/v1/webhooks/:id
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/webhooks/{webhook_id}"))
        .header("Authorization", admin_auth)
        .body(Body::empty())?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // 4. Verify list is now empty for active webhooks
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/webhooks")
        .header("Authorization", admin_auth)
        .body(Body::empty())?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), 1024 * 64).await?;
    let list: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(list.as_array().map(|a| a.len()), Some(0));

    Ok(())
}

#[tokio::test]
async fn test_hmac_signatures_and_payload_formatters() -> Result<(), Box<dyn Error>> {
    let secret = "super-secret-hmac-key";
    let body = b"{\"event\":\"snapshot_pushed\",\"repo_id\":\"repo1\"}";

    // Generate and verify HMAC
    let sig = generate_hmac_signature(secret, body);
    assert!(!sig.is_empty());

    let is_valid = verify_hmac_signature(secret, body, &format!("sha256={sig}"));
    assert!(is_valid);

    let is_valid_no_prefix = verify_hmac_signature(secret, body, &sig);
    assert!(is_valid_no_prefix);

    let is_invalid = verify_hmac_signature(secret, body, "sha256=invalidhex0123");
    assert!(!is_invalid);

    let is_wrong_secret = verify_hmac_signature("wrong-key", body, &sig);
    assert!(!is_wrong_secret);

    // Payload formatting for Slack & Discord
    let payload = WebhookPayload {
        event: WebhookEvent::CriticalVulnerability,
        repo_id: "elcode/forge".to_string(),
        branch: "main".to_string(),
        commit_hash: "1234567890abcdef".to_string(),
        health_score: HealthScore {
            overall: 65,
            dimensions: Vec::new(),
        },
        delta: None,
        critical_findings: vec![sample_critical_finding("CRIT-1")],
        timestamp_secs: 1_700_000_000,
        message: "Discovered 1 critical vulnerability".to_string(),
    };

    let slack_json = format_slack_payload(&payload);
    assert!(slack_json["blocks"].is_array());
    assert!(slack_json["text"].as_str().is_some());

    let discord_json = format_discord_payload(&payload);
    assert!(discord_json["embeds"].is_array());
    assert_eq!(discord_json["embeds"][0]["color"], 0xE02424);

    Ok(())
}

#[tokio::test]
async fn test_webhook_dispatch_on_push_event() -> Result<(), Box<dyn Error>> {
    // 1. Setup local mock HTTP receiver for webhook delivery
    let received_count = Arc::new(AtomicUsize::new(0));
    let received_sig = Arc::new(tokio::sync::Mutex::new(String::new()));
    let received_body = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    let rc_clone = Arc::clone(&received_count);
    let sig_clone = Arc::clone(&received_sig);
    let body_clone = Arc::clone(&received_body);

    let mock_receiver = axum::Router::new().route(
        "/mock-webhook",
        axum::routing::post(move |req: axum::extract::Request| {
            let rc = Arc::clone(&rc_clone);
            let sc = Arc::clone(&sig_clone);
            let bc = Arc::clone(&body_clone);
            async move {
                let sig = req
                    .headers()
                    .get("X-Forge-Signature")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_string();

                let body_bytes = to_bytes(req.into_body(), 1024 * 64)
                    .await
                    .unwrap_or_default()
                    .to_vec();

                let mut s = sc.lock().await;
                *s = sig;
                let mut b = bc.lock().await;
                *b = body_bytes;

                rc.fetch_add(1, Ordering::SeqCst);
                StatusCode::OK
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let mock_addr = listener.local_addr()?;
    tokio::spawn(async move {
        let _ = axum::serve(listener, mock_receiver).await;
    });

    let webhook_url = format!("http://127.0.0.1:{}/mock-webhook", mock_addr.port());
    let webhook_secret = "webhook-test-shared-secret-key";

    // 2. Setup Forge Server app and register webhook
    let (app, _) = setup_test_app().await?;
    let admin_auth = "Bearer test-admin-secret-key-12345";

    let reg_body = serde_json::json!({
        "repo_id": "monitored-repo",
        "url": webhook_url,
        "secret": webhook_secret,
        "events": ["snapshot_pushed", "critical_vulnerability"]
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/webhooks")
        .header("Authorization", admin_auth)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&reg_body)?))?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::CREATED);

    // 3. Push a snapshot containing a critical finding
    let push = sample_push(
        "monitored-repo",
        "Monitored Repository",
        "main",
        "deadbeef12345678",
        75,
        vec![sample_critical_finding("F-CVE-999")],
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sync/push")
        .header("Authorization", admin_auth)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&push)?))?;
    let res = app.clone().oneshot(req).await?;
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Wait up to 2 seconds for the background async dispatch to deliver webhook
    let mut received = false;
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if received_count.load(Ordering::SeqCst) > 0 {
            received = true;
            break;
        }
    }

    assert!(
        received,
        "Expected mock webhook receiver to receive notification within 2s"
    );

    // 5. Verify HMAC-SHA256 signature on received body
    let sig = received_sig.lock().await.clone();
    let body = received_body.lock().await.clone();

    assert!(
        sig.starts_with("sha256="),
        "Expected signature with sha256= prefix, got: {sig}"
    );
    assert!(
        verify_hmac_signature(webhook_secret, &body, &sig),
        "Webhook HMAC-SHA256 signature verification failed!"
    );

    Ok(())
}
