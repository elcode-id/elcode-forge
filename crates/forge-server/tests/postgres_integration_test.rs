//! Integration test for PostgreSQL backend when FORGE_DATABASE_URL is provided (e.g. in CI service containers).

use std::collections::BTreeMap;
use std::error::Error;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use forge_core::ir::{Confidence, DimensionScore, Finding, HealthScore, Severity, SourceLocation};
use forge_core::sync::{SyncManifest, SyncPushPayload};
use forge_server::{create_app, AppState, Db, ServerConfig};

fn get_postgres_url() -> Option<String> {
    std::env::var("FORGE_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .filter(|u| u.starts_with("postgres:") || u.starts_with("postgresql:"))
}

fn sample_push(
    repo_id: &str,
    repo_name: &str,
    branch: &str,
    commit: &str,
    overall_health: u8,
) -> SyncPushPayload {
    let manifest = SyncManifest {
        repo_id: repo_id.to_string(),
        repo_name: repo_name.to_string(),
        branch: branch.to_string(),
        commit_hash: commit.to_string(),
        timestamp_secs: chrono::Utc::now().timestamp().max(0) as u64,
        content_hash: format!("pg_hash_{commit}"),
        tool_version: "1.0.0".to_string(),
    };

    let health_score = HealthScore {
        overall: overall_health,
        dimensions: vec![DimensionScore {
            dimension: "architecture".to_string(),
            score: overall_health,
            weight_pct: 100,
            reasons: Vec::new(),
        }],
    };

    let finding = Finding {
        id: format!("pg-sec-{commit}"),
        rule_id: "sec-pg-001".to_string(),
        category: "security".to_string(),
        severity: Severity::High,
        confidence: Confidence::Certain,
        title: "Test finding in PostgreSQL".to_string(),
        description: "Details regarding test finding".to_string(),
        evidence: "evidence text".to_string(),
        location: Some(SourceLocation {
            file_path: "crates/db.rs".to_string(),
            start_line: Some(10),
            end_line: Some(15),
        }),
        recommendation: "Fix sql parameterization".to_string(),
    };

    SyncPushPayload {
        manifest,
        findings_count: 1,
        health_score,
        findings: vec![finding],
        modules: Vec::new(),
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
async fn test_postgres_backend_lifecycle() -> Result<(), Box<dyn Error>> {
    let pg_url = match get_postgres_url() {
        Some(url) => url,
        None => {
            eprintln!("Skipping PostgreSQL integration test: FORGE_DATABASE_URL with postgres scheme is not set.");
            return Ok(());
        }
    };

    println!("Connecting to PostgreSQL at {pg_url}...");
    let db = Db::connect(&pg_url).await?;
    assert_eq!(db.backend_name(), "postgres");

    println!("Running PostgreSQL migrations...");
    db.migrate().await?;

    let config = ServerConfig {
        admin_api_key: Some("pg-admin-secret-key-12345".to_string()),
        jwt_secret: "pg-jwt-secret-string-at-least-32-characters-long".to_string(),
        ..Default::default()
    };

    let state = AppState::new(db.clone(), config);
    let app = create_app(state);

    let auth_header = "Bearer pg-admin-secret-key-12345";
    let project_id = "test-pg-repo";

    // 1. Push snapshot to PostgreSQL
    let push_payload = sample_push(project_id, "Test Postgres Project", "main", "abc0001", 94);
    let push_req = Request::builder()
        .method("POST")
        .uri("/api/v1/sync/push")
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&push_payload)?))?;

    let push_res = app.clone().oneshot(push_req).await?;
    assert_eq!(push_res.status(), StatusCode::OK);

    // 2. Query project registry
    let projects_req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects")
        .header("Authorization", auth_header)
        .body(Body::empty())?;

    let projects_res = app.clone().oneshot(projects_req).await?;
    assert_eq!(projects_res.status(), StatusCode::OK);
    let projects_bytes = to_bytes(projects_res.into_body(), usize::MAX).await?;
    let projects: Vec<serde_json::Value> = serde_json::from_slice(&projects_bytes)?;
    assert!(projects.iter().any(|p| p["repo_id"] == project_id));

    // 3. Query branches
    let branches_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{project_id}/branches"))
        .header("Authorization", auth_header)
        .body(Body::empty())?;

    let branches_res = app.clone().oneshot(branches_req).await?;
    assert_eq!(branches_res.status(), StatusCode::OK);
    let branches_bytes = to_bytes(branches_res.into_body(), usize::MAX).await?;
    let branches: Vec<serde_json::Value> = serde_json::from_slice(&branches_bytes)?;
    assert!(branches.iter().any(|b| b["branch"] == "main"));

    // 4. Query history
    let history_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{project_id}/history?branch=main"))
        .header("Authorization", auth_header)
        .body(Body::empty())?;

    let history_res = app.clone().oneshot(history_req).await?;
    assert_eq!(history_res.status(), StatusCode::OK);
    let history_bytes = to_bytes(history_res.into_body(), usize::MAX).await?;
    let history: serde_json::Value = serde_json::from_slice(&history_bytes)?;
    assert_eq!(history["total_points"], 1);
    assert_eq!(history["points"].as_array().map(|a| a.len()), Some(1));

    // 5. Pull snapshot
    let pull_req = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/v1/sync/pull?repo_id={project_id}&branch=main"
        ))
        .header("Authorization", auth_header)
        .body(Body::empty())?;

    let pull_res = app.oneshot(pull_req).await?;
    assert_eq!(pull_res.status(), StatusCode::OK);
    let pull_bytes = to_bytes(pull_res.into_body(), usize::MAX).await?;
    let pull_json: serde_json::Value = serde_json::from_slice(&pull_bytes)?;
    assert_eq!(pull_json["repo_id"], project_id);
    assert_eq!(pull_json["baseline_commit_hash"], "abc0001");

    Ok(())
}
