//! Integration tests for enterprise JWKS/proxy auth, RBAC roles, and append-only audit event logging.

use std::collections::BTreeMap;
use std::error::Error;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use forge_core::ir::{DimensionScore, HealthScore};
use forge_core::sync::{SyncManifest, SyncPushPayload};
use forge_server::{
    create_app, issue_jwt, AppState, AuditEventRow, Db, JwkKey, JwksDocument, ServerConfig,
};

async fn setup_app(trust_proxy: bool) -> Result<(axum::Router, AppState), Box<dyn Error>> {
    let db = Db::connect_in_memory().await?;
    let config = ServerConfig {
        admin_api_key: Some("admin-secret-key-xyz".to_string()),
        jwt_secret: "secret-jwt-key-minimum-32-chars-long-here".to_string(),
        trust_proxy_headers: trust_proxy,
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
        dimensions: vec![DimensionScore {
            dimension: "security".to_string(),
            score: overall_health,
            weight_pct: 100,
            reasons: Vec::new(),
        }],
    };

    SyncPushPayload {
        manifest,
        findings_count: 0,
        health_score,
        findings: Vec::new(),
        modules: Vec::new(),
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
async fn test_proxy_headers_security_boundary() -> Result<(), Box<dyn Error>> {
    // 1. When trust_proxy_headers is false, reverse-proxy headers must be ignored
    let (app_untrusted, _state) = setup_app(false).await?;

    let spoof_req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects")
        .header("X-Forwarded-User", "hacker")
        .header("X-Forwarded-Role", "admin")
        .body(Body::empty())?;

    let resp = app_untrusted.oneshot(spoof_req).await?;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 2. When trust_proxy_headers is true, reverse-proxy headers authenticate the user
    let (app_trusted, _state) = setup_app(true).await?;

    let proxy_req = Request::builder()
        .method("GET")
        .uri("/api/v1/projects")
        .header("X-Forwarded-User", "corp_employee")
        .header("X-Forwarded-Role", "auditor")
        .header("X-Forwarded-Email", "employee@corp.internal")
        .body(Body::empty())?;

    let resp = app_trusted.oneshot(proxy_req).await?;
    assert_eq!(resp.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_rbac_audit_endpoint_authorization() -> Result<(), Box<dyn Error>> {
    let (app, state) = setup_app(false).await?;

    // Issue tokens for various roles
    let viewer_token = issue_jwt(&state.config.jwt_secret, "alice", "viewer", 3600)?;
    let maintainer_token = issue_jwt(&state.config.jwt_secret, "bob", "maintainer", 3600)?;
    let auditor_token = issue_jwt(&state.config.jwt_secret, "carol", "auditor", 3600)?;
    let admin_token = issue_jwt(&state.config.jwt_secret, "dave", "admin", 3600)?;

    // 1. Viewer cannot view audit events -> 403 Forbidden
    let req_viewer = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-events")
        .header("Authorization", format!("Bearer {viewer_token}"))
        .body(Body::empty())?;
    let resp = app.clone().oneshot(req_viewer).await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 2. Maintainer cannot view audit events -> 403 Forbidden
    let req_maintainer = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-events")
        .header("Authorization", format!("Bearer {maintainer_token}"))
        .body(Body::empty())?;
    let resp = app.clone().oneshot(req_maintainer).await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 3. Auditor can view audit events -> 200 OK
    let req_auditor = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-events")
        .header("Authorization", format!("Bearer {auditor_token}"))
        .body(Body::empty())?;
    let resp = app.clone().oneshot(req_auditor).await?;
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. Admin can view audit events -> 200 OK
    let req_admin = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-events")
        .header("Authorization", format!("Bearer {admin_token}"))
        .body(Body::empty())?;
    let resp = app.oneshot(req_admin).await?;
    assert_eq!(resp.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_audit_event_lifecycle_and_filters() -> Result<(), Box<dyn Error>> {
    let (app, state) = setup_app(false).await?;
    let admin_token = issue_jwt(&state.config.jwt_secret, "admin_user", "admin", 3600)?;
    let auth_header = format!("Bearer {admin_token}");

    // 1. Create Organization "SecOps"
    let org_payload = serde_json::json!({
        "name": "SecOps Enterprise",
        "slug": "secops"
    });
    let org_req = Request::builder()
        .method("POST")
        .uri("/api/v1/orgs")
        .header("Content-Type", "application/json")
        .header("Authorization", &auth_header)
        .body(Body::from(serde_json::to_vec(&org_payload)?))?;
    let org_resp = app.clone().oneshot(org_req).await?;
    assert_eq!(org_resp.status(), StatusCode::CREATED);
    let org_body = to_bytes(org_resp.into_body(), usize::MAX).await?;
    let org: serde_json::Value = serde_json::from_slice(&org_body)?;
    let org_id = org["id"].as_str().unwrap_or_default();

    // 2. Link Project "secops_gateway"
    let link_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/orgs/{org_id}/projects"))
        .header("Content-Type", "application/json")
        .header("Authorization", &auth_header)
        .body(Body::from(serde_json::to_vec(
            &serde_json::json!({ "project_id": "secops_gateway" }),
        )?))?;
    let link_resp = app.clone().oneshot(link_req).await?;
    assert_eq!(link_resp.status(), StatusCode::OK);

    // 3. Create Webhook
    let hook_payload = serde_json::json!({
        "url": "https://hooks.secops.internal/events",
        "repo_id": "secops_gateway"
    });
    let hook_req = Request::builder()
        .method("POST")
        .uri("/api/v1/webhooks")
        .header("Content-Type", "application/json")
        .header("Authorization", &auth_header)
        .body(Body::from(serde_json::to_vec(&hook_payload)?))?;
    let hook_resp = app.clone().oneshot(hook_req).await?;
    assert_eq!(hook_resp.status(), StatusCode::CREATED);
    let hook_body = to_bytes(hook_resp.into_body(), usize::MAX).await?;
    let hook: serde_json::Value = serde_json::from_slice(&hook_body)?;
    let hook_id = hook["id"].as_str().unwrap_or_default();

    // 4. Ingest Snapshot
    let push = sample_push("secops_gateway", "SecOps Gateway", "main", "commit_001", 95);
    let push_req = Request::builder()
        .method("POST")
        .uri("/api/v1/sync/push")
        .header("Content-Type", "application/json")
        .header("Authorization", &auth_header)
        .body(Body::from(serde_json::to_vec(&push)?))?;
    let push_resp = app.clone().oneshot(push_req).await?;
    assert_eq!(push_resp.status(), StatusCode::OK);

    // 5. Delete Webhook
    let del_req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/webhooks/{hook_id}"))
        .header("Authorization", &auth_header)
        .body(Body::empty())?;
    let del_resp = app.clone().oneshot(del_req).await?;
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    // Give background async tasks a brief moment to complete persistence
    tokio::time::sleep(Duration::from_millis(60)).await;

    // 6. Query all audit events
    let query_req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-events")
        .header("Authorization", &auth_header)
        .body(Body::empty())?;
    let query_resp = app.clone().oneshot(query_req).await?;
    assert_eq!(query_resp.status(), StatusCode::OK);

    let query_body = to_bytes(query_resp.into_body(), usize::MAX).await?;
    let events: Vec<AuditEventRow> = serde_json::from_slice(&query_body)?;

    // We recorded: org.create, project.link, webhook.create, snapshot.push, webhook.delete
    assert!(events.len() >= 5);

    let actions: Vec<&str> = events.iter().map(|e| e.action.as_str()).collect();
    assert!(actions.contains(&"org.create"));
    assert!(actions.contains(&"project.link"));
    assert!(actions.contains(&"webhook.create"));
    assert!(actions.contains(&"snapshot.push"));
    assert!(actions.contains(&"webhook.delete"));

    // 7. Filter by action: snapshot.push
    let filter_req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-events?action=snapshot.push")
        .header("Authorization", &auth_header)
        .body(Body::empty())?;
    let filter_resp = app.clone().oneshot(filter_req).await?;
    assert_eq!(filter_resp.status(), StatusCode::OK);

    let filter_body = to_bytes(filter_resp.into_body(), usize::MAX).await?;
    let filtered_events: Vec<AuditEventRow> = serde_json::from_slice(&filter_body)?;
    assert_eq!(filtered_events.len(), 1);
    assert_eq!(filtered_events[0].action, "snapshot.push");
    assert_eq!(filtered_events[0].actor, "admin_user");
    assert_eq!(
        filtered_events[0].resource_id,
        "secops_gateway:main:commit_001"
    );

    // 8. Filter by actor: admin_user
    let actor_req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-events?actor=admin_user&limit=2")
        .header("Authorization", &auth_header)
        .body(Body::empty())?;
    let actor_resp = app.oneshot(actor_req).await?;
    assert_eq!(actor_resp.status(), StatusCode::OK);

    let actor_body = to_bytes(actor_resp.into_body(), usize::MAX).await?;
    let actor_events: Vec<AuditEventRow> = serde_json::from_slice(&actor_body)?;
    assert_eq!(actor_events.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_jwks_cache_static_key_verification() -> Result<(), Box<dyn Error>> {
    let (_app, state) = setup_app(false).await?;

    // 1. Symmetric key loaded into JWKS must be rejected (algorithm confusion defense)
    let kid = "static-test-key-001";
    let key_secret = "static-jwk-secret-key-that-is-at-least-32-bytes";

    let jwk = JwkKey {
        kty: "oct".to_string(),
        kid: Some(kid.to_string()),
        alg: Some("HS256".to_string()),
        n: None,
        e: None,
        k: Some(key_secret.to_string()),
    };

    let doc = JwksDocument { keys: vec![jwk] };
    state.jwks.load_static_keys(doc).await;

    // Issue token with kid in header
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = forge_server::Claims {
        sub: "jwks_auditor".to_string(),
        role: "auditor".to_string(),
        exp: now + 3600,
        iat: now,
    };

    let header = jsonwebtoken::Header {
        kid: Some(kid.to_string()),
        ..Default::default()
    };

    let token = jsonwebtoken::encode(
        &header,
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(key_secret.as_bytes()),
    )?;

    // Verify token directly against JwksCache: must be rejected with ServerError::Auth
    let res = state.jwks.verify_token(&token, None).await;
    assert!(res.is_err(), "symmetric HS256 tokens must be rejected");

    // 2. Asymmetric RSA key loaded into JWKS
    let rsa_kid = "rsa-test-key-002";
    let rsa_n = "1eTlCM0RzKpt7--ONr5L5DZxkQVNWh5-765386_pZQUEaquTzs6yQwXD0jUf3IHw4VEro76IhjGC53v4TNV93nFswDiUvRcOy5jeHHWjsGy0swgAB4fkihxKp8PJm3oibKAzQqZWcqlruqYDvPv74BeUhplUz9yU_nXBD1EBNSjN08S_InFQrDu7rfhGC3aV5ixpRJcqYZyaAs20DzlmaDMnd4c8t_uVXnu6M39EZof4FH4Ad1_IdhnwGgclkEuF5q8BPXIzvYn0g9c338aSvcr-qLYIuzuhkDWkJ-WJhVZI036h1-aVcIEelu-MUQZnGYH8RVHIGC6FNzhYFsHsbQ";
    let rsa_e = "AQAB";

    let rsa_jwk = JwkKey {
        kty: "RSA".to_string(),
        kid: Some(rsa_kid.to_string()),
        alg: Some("RS256".to_string()),
        n: Some(rsa_n.to_string()),
        e: Some(rsa_e.to_string()),
        k: None,
    };
    state
        .jwks
        .load_static_keys(JwksDocument {
            keys: vec![rsa_jwk],
        })
        .await;

    let rsa_pem = b"-----BEGIN PRIVATE KEY-----\nMIIEuwIBADANBgkqhkiG9w0BAQEFAASCBKUwggShAgEAAoIBAQDV5OUIzRHMqm3v\n7442vkvkNnGRBU1aHn7vrnfzr+llBQRqq5POzrJDBcPSNR/cgfDhUSujvoiGMYLn\ne/hM1X3ecWzAOJS9Fw7LmN4cdaOwbLSzCAAHh+SKHEqnw8mbeiJsoDNCplZyqWu6\npgO8+/vgF5SGmVTP3JT+dcEPUQE1KM3TxL8icVCsO7ut+EYLdpXmLGlElyphnJoC\nzbQPOWZoMyd3hzy3+5Vee7ozf0Rmh/gUfgB3X8h2GfAaByWQS4XmrwE9cjO9ifSD\n1zffxpK9yv6otgi7O6GQNaQn5YmFVkjTfqHX5pVwgR6W74xRBmcZgfxFUcgYLoU3\nOFgWwextAgMBAAECggEARCZO9zPnlO9EYetQw450Pb0/B/bJg4ot5Y4QN/t3gLFN\ni/Mo//6Je6NNlh9JDIf2SMQFBIIDm8YQsFKmdUyccxBulvwwH+NF2obVVfoBcofg\nNx64a63Xmk2TV8qg5MHQUA3m5zh6WMQ0R973V/JteSPqNcyK4kOk0UKRhaDIorfN\nUpm9doN21Ggj2FgXVnoi83CVz+g5o0hSTAlbwTi3Lq4Idn2Clc3nC88S7bs7ac8W\nBVUReS0ut9vm6dQYOslHqGqrWHOBkDNqZHIRtTB+v6aKiJ3v1c5Hs7jIRaLFy2e5\nNxmHIjTc7ILKcBOZbSKbqQLScJo56+z7hPBb9N+PHQKBgQDx2qdcVa21MXBgmiCO\ns7/z2iFrcd6DHiYydMp/8xyl2G1GQoExrxw/5ZvhWphkbeq8lEEB1b86cJz0c19G\ndt3mz/j05HuVt1g0cRySJSNStmUSuR8+WwyUfWGRlmpxkSK1qGmqr1GprvwaShtG\n738OYaisIwt3wR10rb/QiDM73wKBgQDiZ5ai1JleG6IMhacMTzL+H9xW3HFndN7g\nbvZYbc6NpCV4AqXWnpjgtkIs0WvbVh1c/W+Om/R82EDvwxxp8djK32osoGEeiV0h\nh3mU+pIiz8iQG2A0JTyOKJV5KqsZJziU4wiH8IRbK3YY2XNJ5VjlZ98OsoReBTVJ\ni/YJMNLhMwJ/XJVXfAFQfgxu+IwH6pjk4nSeVoNAqvDfrnEHE/P9A3/WVQ8gZeSi\n69L52UrVC50IPyiBujIZAkYZz3pa/p6CU9lJdHYC2U1WdrzWMxwEaaVpC8HSNAym\nxDE7KIBsPw1QPDo8ORrkKBDUcTvtNTp5aQf5ievgd/d1TzU0OJ/q/QKBgFAfar7b\nUB9u1g1qazC/AoPOkLKotvVu9cUlX8bxrUyRD7WF8lyeqem0dnzX7DymICQdiJJG\nfOKFwY1YVkDkFt3M7Q04AMbGxA+umwdvGbJ6KDdfiM9Lcj+4+xTzVbPEHpEKO6Wo\nJ58AJX4rzlITeT9OYDEaWh/P5gjOIFcmUSrjAoGBAO+S+JzPEIo/2QVknZDIfKiR\nbER7YO+EyCWNtnKLUAc7Lyvbn9Jo02hTJmgnNMRP3BiQMxWLrtcyZEemvHNonB8t\nu3FAR+dqeDIPIqdupWBBLBAfYbg7lxdCaXxIX19avIXrRq9mwHqO/VwrhvJNsjV/\n+u3Trui+5eFqBryOxLHY\n-----END PRIVATE KEY-----";

    let rsa_header = jsonwebtoken::Header {
        alg: jsonwebtoken::Algorithm::RS256,
        kid: Some(rsa_kid.to_string()),
        ..Default::default()
    };
    let encoding_key = jsonwebtoken::EncodingKey::from_rsa_pem(rsa_pem)?;
    let rsa_token = jsonwebtoken::encode(&rsa_header, &claims, &encoding_key)?;

    let verified_claims = state.jwks.verify_token(&rsa_token, None).await?;
    assert_eq!(verified_claims.sub, "jwks_auditor");
    assert_eq!(verified_claims.role, "auditor");

    Ok(())
}
