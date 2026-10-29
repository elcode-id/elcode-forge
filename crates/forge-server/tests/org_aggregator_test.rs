//! Integration tests for organization aggregator and materialized rollups.

use std::collections::BTreeMap;
use std::error::Error;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use forge_core::ir::{Confidence, DimensionScore, Finding, HealthScore, Severity, SourceLocation};
use forge_core::sync::{SyncManifest, SyncPushPayload};
use forge_server::{
    create_app, AppState, CrossRepoVulnerability, Db, OrganizationRow, OrganizationSummary,
    ServerConfig,
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

fn sample_finding(
    id: &str,
    rule_id: &str,
    category: &str,
    severity: Severity,
    title: &str,
) -> Finding {
    Finding {
        id: id.to_string(),
        rule_id: rule_id.to_string(),
        category: category.to_string(),
        severity,
        confidence: Confidence::Certain,
        title: title.to_string(),
        description: format!("Description for {title}"),
        location: Some(SourceLocation {
            file_path: "src/security.ts".to_string(),
            start_line: Some(10),
            end_line: Some(15),
        }),
        evidence: "insecure_call(...)".to_string(),
        recommendation: "Harden input".to_string(),
    }
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
        tool_version: "1.0.0".to_string(),
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

#[tokio::test]
async fn test_org_lifecycle_and_creation() -> Result<(), Box<dyn Error>> {
    let (app, _state) = setup_test_app().await?;

    // 1. Create Organization via POST /api/v1/orgs
    let create_payload = serde_json::json!({
        "name": "Acme Technologies",
        "slug": "acme"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/orgs")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::from(serde_json::to_vec(&create_payload)?))?;

    let resp = app.clone().oneshot(req).await?;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body(), usize::MAX).await?;
    let org_row: OrganizationRow = serde_json::from_slice(&body)?;
    assert_eq!(org_row.name, "Acme Technologies");
    assert_eq!(org_row.slug, "acme");
    assert!(org_row.id.starts_with("org_"));

    // 2. List Organizations via GET /api/v1/orgs
    let list_req = Request::builder()
        .method("GET")
        .uri("/api/v1/orgs")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::empty())?;

    let list_resp = app.clone().oneshot(list_req).await?;
    assert_eq!(list_resp.status(), StatusCode::OK);

    let list_body = to_bytes(list_resp.into_body(), usize::MAX).await?;
    let list: Vec<serde_json::Value> = serde_json::from_slice(&list_body)?;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["slug"], "acme");
    assert_eq!(list[0]["total_projects"], 0);
    assert_eq!(list[0]["health_score"], 100);

    // 3. Fetch initial summary via GET /api/v1/orgs/acme/summary
    let summary_req = Request::builder()
        .method("GET")
        .uri("/api/v1/orgs/acme/summary")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::empty())?;

    let summary_resp = app.oneshot(summary_req).await?;
    assert_eq!(summary_resp.status(), StatusCode::OK);

    let summary_body = to_bytes(summary_resp.into_body(), usize::MAX).await?;
    let summary: OrganizationSummary = serde_json::from_slice(&summary_body)?;
    assert_eq!(summary.slug, "acme");
    assert_eq!(summary.health_score, 100);
    assert_eq!(summary.total_projects, 0);
    assert_eq!(summary.total_vulnerabilities, 0);
    assert!(summary.cross_repo_vulnerabilities.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_link_projects_and_cross_repo_vulnerability_matrix() -> Result<(), Box<dyn Error>> {
    let (app, _state) = setup_test_app().await?;

    // 1. Create Organization "Fintech Core"
    let create_payload = serde_json::json!({
        "name": "Fintech Core",
        "slug": "fintech"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/orgs")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::from(serde_json::to_vec(&create_payload)?))?;
    let resp = app.clone().oneshot(req).await?;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body(), usize::MAX).await?;
    let org: OrganizationRow = serde_json::from_slice(&body)?;

    // 2. Ingest snapshot for repo_payments (Health 90, shared CVE-2023-AUTH)
    let f1 = sample_finding(
        "F-AUTH-1",
        "sec/cve-2023-auth",
        "security",
        Severity::High,
        "Broken Authentication in OAuth Middleware",
    );
    let push_payments = sample_push(
        "repo_payments",
        "Payments Service",
        "main",
        "c011111",
        90,
        vec![f1.clone()],
    );
    let push_req = Request::builder()
        .method("POST")
        .uri("/api/v1/sync/push")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::from(serde_json::to_vec(&push_payments)?))?;
    let push_resp = app.clone().oneshot(push_req).await?;
    assert_eq!(push_resp.status(), StatusCode::OK);

    // 3. Ingest snapshot for repo_ledger (Health 70, shared CVE-2023-AUTH and unique SQL-INJ)
    let f2 = sample_finding(
        "F-AUTH-2",
        "sec/cve-2023-auth",
        "security",
        Severity::High,
        "Broken Authentication in OAuth Middleware",
    );
    let f3 = sample_finding(
        "F-SQL-1",
        "sec/sql-injection",
        "security",
        Severity::Critical,
        "SQL Injection via Unsanitized Statement",
    );
    let push_ledger = sample_push(
        "repo_ledger",
        "Ledger Service",
        "main",
        "c022222",
        70,
        vec![f2, f3],
    );
    let push_req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/sync/push")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::from(serde_json::to_vec(&push_ledger)?))?;
    let push_resp2 = app.clone().oneshot(push_req2).await?;
    assert_eq!(push_resp2.status(), StatusCode::OK);

    // 4. Link repo_payments to fintech
    let link_req1 = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/orgs/{}/projects", org.id))
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::from(serde_json::to_vec(
            &serde_json::json!({ "project_id": "repo_payments" }),
        )?))?;
    let link_resp1 = app.clone().oneshot(link_req1).await?;
    assert_eq!(link_resp1.status(), StatusCode::OK);

    // 5. Link repo_ledger to fintech
    let link_req2 = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/orgs/{}/projects", org.id))
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::from(serde_json::to_vec(
            &serde_json::json!({ "project_id": "repo_ledger" }),
        )?))?;
    let link_resp2 = app.clone().oneshot(link_req2).await?;
    assert_eq!(link_resp2.status(), StatusCode::OK);

    // 6. Query Organization Summary
    let summary_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/orgs/{}/summary", org.slug))
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::empty())?;
    let summary_resp = app.clone().oneshot(summary_req).await?;
    assert_eq!(summary_resp.status(), StatusCode::OK);

    let summary_body = to_bytes(summary_resp.into_body(), usize::MAX).await?;
    let summary: OrganizationSummary = serde_json::from_slice(&summary_body)?;

    assert_eq!(summary.total_projects, 2);
    // Average of 90 and 70 is 80
    assert_eq!(summary.health_score, 80);
    // Total findings across portfolio: 1 in payments + 2 in ledger = 3
    assert_eq!(summary.total_vulnerabilities, 3);
    assert_eq!(summary.cross_repo_vulnerabilities.len(), 2);

    // The shared auth vulnerability should be top of matrix (affected_projects: 2)
    let top_vuln = &summary.cross_repo_vulnerabilities[0];
    assert_eq!(top_vuln.rule_id, "sec/cve-2023-auth");
    assert_eq!(top_vuln.occurrence_count, 2);
    assert_eq!(top_vuln.affected_projects.len(), 2);
    assert!(top_vuln
        .affected_projects
        .contains(&"repo_payments".to_string()));
    assert!(top_vuln
        .affected_projects
        .contains(&"repo_ledger".to_string()));

    // The unique SQL injection vulnerability
    let second_vuln = &summary.cross_repo_vulnerabilities[1];
    assert_eq!(second_vuln.rule_id, "sec/sql-injection");
    assert_eq!(second_vuln.occurrence_count, 1);
    assert_eq!(
        second_vuln.affected_projects,
        vec!["repo_ledger".to_string()]
    );

    // 7. Verify /vulnerabilities endpoint directly
    let vuln_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/orgs/{}/vulnerabilities", org.id))
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::empty())?;
    let vuln_resp = app.oneshot(vuln_req).await?;
    assert_eq!(vuln_resp.status(), StatusCode::OK);

    let vuln_body = to_bytes(vuln_resp.into_body(), usize::MAX).await?;
    let matrix: Vec<CrossRepoVulnerability> = serde_json::from_slice(&vuln_body)?;
    assert_eq!(matrix.len(), 2);
    assert_eq!(matrix[0].rule_id, "sec/cve-2023-auth");

    Ok(())
}

#[tokio::test]
async fn test_push_triggers_automatic_rollup_recomputation() -> Result<(), Box<dyn Error>> {
    let (app, _state) = setup_test_app().await?;

    // 1. Create organization
    let create_payload = serde_json::json!({
        "name": "Cloud Native Org",
        "slug": "cloud"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/orgs")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::from(serde_json::to_vec(&create_payload)?))?;
    let resp = app.clone().oneshot(req).await?;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 2. Pre-link project repo_cloud before any snapshot is pushed
    let link_req = Request::builder()
        .method("POST")
        .uri("/api/v1/orgs/cloud/projects")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::from(serde_json::to_vec(
            &serde_json::json!({ "project_id": "repo_cloud" }),
        )?))?;
    let link_resp = app.clone().oneshot(link_req).await?;
    assert_eq!(link_resp.status(), StatusCode::OK);

    // 3. Initial summary check: 1 project linked, but 0 snapshots pushed -> health 100
    let summary_req = Request::builder()
        .method("GET")
        .uri("/api/v1/orgs/cloud/summary")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::empty())?;
    let summary_resp = app.clone().oneshot(summary_req).await?;
    assert_eq!(summary_resp.status(), StatusCode::OK);
    let body = to_bytes(summary_resp.into_body(), usize::MAX).await?;
    let summary: OrganizationSummary = serde_json::from_slice(&body)?;
    assert_eq!(summary.health_score, 100);
    assert_eq!(summary.total_vulnerabilities, 0);

    // 4. Ingest snapshot for repo_cloud (Health 65, with CVE)
    let f1 = sample_finding(
        "F-RCE-1",
        "sec/cve-2024-rce",
        "security",
        Severity::Critical,
        "Remote Code Execution via YAML Deserialization",
    );
    let push_payload = sample_push(
        "repo_cloud",
        "Cloud Service",
        "main",
        "c033333",
        65,
        vec![f1],
    );
    let push_req = Request::builder()
        .method("POST")
        .uri("/api/v1/sync/push")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::from(serde_json::to_vec(&push_payload)?))?;
    let push_resp = app.clone().oneshot(push_req).await?;
    assert_eq!(push_resp.status(), StatusCode::OK);

    // Give background async task a moment to finalize rollup recomputation
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 5. Query summary again: rollup must be automatically updated!
    let summary_req2 = Request::builder()
        .method("GET")
        .uri("/api/v1/orgs/cloud/summary")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::empty())?;
    let summary_resp2 = app.oneshot(summary_req2).await?;
    assert_eq!(summary_resp2.status(), StatusCode::OK);
    let body2 = to_bytes(summary_resp2.into_body(), usize::MAX).await?;
    let summary2: OrganizationSummary = serde_json::from_slice(&body2)?;

    assert_eq!(summary2.health_score, 65);
    assert_eq!(summary2.total_vulnerabilities, 1);
    assert_eq!(summary2.cross_repo_vulnerabilities.len(), 1);
    assert_eq!(
        summary2.cross_repo_vulnerabilities[0].rule_id,
        "sec/cve-2024-rce"
    );

    Ok(())
}

#[tokio::test]
async fn test_validation_errors() -> Result<(), Box<dyn Error>> {
    let (app, _state) = setup_test_app().await?;

    // 1. Empty name should be rejected with 400
    let empty_name = serde_json::json!({
        "name": "   ",
        "slug": "invalid"
    });
    let req1 = Request::builder()
        .method("POST")
        .uri("/api/v1/orgs")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::from(serde_json::to_vec(&empty_name)?))?;
    let resp1 = app.clone().oneshot(req1).await?;
    assert_eq!(resp1.status(), StatusCode::BAD_REQUEST);

    // 2. Create valid org
    let valid_org = serde_json::json!({
        "name": "Unique Org",
        "slug": "unique-org"
    });
    let req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/orgs")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::from(serde_json::to_vec(&valid_org)?))?;
    let resp2 = app.clone().oneshot(req2).await?;
    assert_eq!(resp2.status(), StatusCode::CREATED);

    // 3. Attempt duplicate slug should fail with 400
    let duplicate_slug = serde_json::json!({
        "name": "Duplicate Org",
        "slug": "unique-org"
    });
    let req3 = Request::builder()
        .method("POST")
        .uri("/api/v1/orgs")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::from(serde_json::to_vec(&duplicate_slug)?))?;
    let resp3 = app.clone().oneshot(req3).await?;
    assert_eq!(resp3.status(), StatusCode::BAD_REQUEST);

    // 4. Requesting non-existent org summary should return 404
    let req4 = Request::builder()
        .method("GET")
        .uri("/api/v1/orgs/non-existent-org-slug/summary")
        .header("Authorization", "Bearer test-admin-secret-key-12345")
        .body(Body::empty())?;
    let resp4 = app.oneshot(req4).await?;
    assert_eq!(resp4.status(), StatusCode::NOT_FOUND);

    Ok(())
}
