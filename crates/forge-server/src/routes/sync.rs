//! Synchronization endpoints for push and pull.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use forge_core::ir::{Finding, HealthScore};
use forge_core::sync::{SyncPullResponse, SyncPushPayload, SyncStatus};

use crate::auth::AuthenticatedUser;
use crate::db::models::{FindingRow, SnapshotRow};
use crate::error::ServerError;
use crate::state::AppState;
use crate::webhooks::{evaluate_and_dispatch_events, EventEvaluationContext};

/// Query parameters for retrieving remote baseline snapshot.
#[derive(Debug, Deserialize)]
pub struct PullQuery {
    pub repo_id: String,
    pub branch: Option<String>,
}

/// Handler for `POST /api/v1/sync/push`.
pub async fn push_handler(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<SyncPushPayload>,
) -> Result<Json<SyncPullResponse>, ServerError> {
    let now = chrono::Utc::now().timestamp();

    // 1. Upsert project
    state
        .db
        .upsert_project(&payload.manifest.repo_id, &payload.manifest.repo_name, now)
        .await?;

    // 2. Query previous latest snapshot to calculate health deltas for webhook notifications
    let previous_snapshot_opt = state
        .db
        .get_latest_snapshot(&payload.manifest.repo_id, &payload.manifest.branch)
        .await
        .ok()
        .flatten();

    let previous_health: Option<HealthScore> = previous_snapshot_opt
        .as_ref()
        .and_then(|(s, _)| serde_json::from_str(&s.health_json).ok());

    // 3. Prepare snapshot row
    let snapshot_id = format!(
        "{}:{}:{}",
        payload.manifest.repo_id, payload.manifest.branch, payload.manifest.commit_hash
    );

    let health_json = serde_json::to_string(&payload.health_score)?;
    let modules_json = Some(serde_json::to_string(&payload.modules)?);
    let metadata_json = Some(serde_json::to_string(&payload.metadata)?);

    let snapshot_row = SnapshotRow {
        id: snapshot_id.clone(),
        repo_id: payload.manifest.repo_id.clone(),
        branch: payload.manifest.branch.clone(),
        commit_hash: payload.manifest.commit_hash.clone(),
        content_hash: payload.manifest.content_hash.clone(),
        tool_version: payload.manifest.tool_version.clone(),
        health_overall: payload.health_score.overall as i64,
        health_json,
        findings_count: payload.findings.len() as i64,
        modules_json,
        metadata_json,
        created_at: now,
    };

    // 4. Prepare finding rows
    let mut finding_rows = Vec::with_capacity(payload.findings.len());
    for (idx, finding) in payload.findings.iter().enumerate() {
        let finding_json = serde_json::to_string(finding)?;
        finding_rows.push(FindingRow {
            id: format!("{snapshot_id}:{idx}:{}", finding.id),
            snapshot_id: snapshot_id.clone(),
            finding_id: finding.id.clone(),
            rule_id: finding.rule_id.clone(),
            category: finding.category.clone(),
            severity: format!("{:?}", finding.severity).to_lowercase(),
            confidence: format!("{:?}", finding.confidence).to_lowercase(),
            title: finding.title.clone(),
            finding_json,
        });
    }

    // 5. Atomically persist snapshot and findings
    state.db.save_snapshot(&snapshot_row, &finding_rows).await?;

    // 6. Evaluate and dispatch outbound webhooks asynchronously
    let ctx = EventEvaluationContext {
        repo_id: &payload.manifest.repo_id,
        branch: &payload.manifest.branch,
        commit_hash: &payload.manifest.commit_hash,
        health_score: &payload.health_score,
        previous_health: previous_health.as_ref(),
        findings: &payload.findings,
        delta: None,
    };
    evaluate_and_dispatch_events(Arc::clone(&state.db), ctx);

    // 7. Asynchronously recompute materialized rollups for all organizations containing this repo
    let org_db = Arc::clone(&state.db);
    let repo_id_for_org = payload.manifest.repo_id.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::org::recompute_all_orgs_for_project(&org_db, &repo_id_for_org).await
        {
            tracing::error!(
                "Failed to recompute organization rollups for project '{repo_id_for_org}': {e}"
            );
        }
    });

    // 8. Record audit event for snapshot ingestion
    let audit_db = Arc::clone(&state.db);
    let audit_actor = user.subject.clone();
    let audit_repo = payload.manifest.repo_id.clone();
    let audit_snap_id = snapshot_id.clone();
    let audit_branch = payload.manifest.branch.clone();
    let audit_commit = payload.manifest.commit_hash.clone();
    let audit_health = payload.health_score.overall;
    let audit_findings_count = payload.findings.len();
    tokio::spawn(async move {
        let org_id = audit_db
            .get_organizations_for_project(&audit_repo)
            .await
            .ok()
            .and_then(|orgs| orgs.into_iter().next())
            .unwrap_or_else(|| "default".to_string());

        let details = serde_json::json!({
            "branch": audit_branch,
            "commit": audit_commit,
            "health": audit_health,
            "findings_count": audit_findings_count
        });

        let _ = crate::audit::record_audit_event(
            &audit_db,
            crate::audit::AuditEventParams {
                org_id: &org_id,
                project_id: Some(&audit_repo),
                actor: &audit_actor,
                action: crate::audit::actions::SNAPSHOT_PUSH,
                resource_type: crate::audit::resources::SNAPSHOT,
                resource_id: &audit_snap_id,
                details: Some(&details),
            },
        )
        .await;
    });

    let response = SyncPullResponse {
        repo_id: payload.manifest.repo_id,
        branch: payload.manifest.branch,
        baseline_commit_hash: Some(payload.manifest.commit_hash),
        baseline_health_score: Some(payload.health_score),
        baseline_findings: payload.findings,
        baseline_content_hash: Some(payload.manifest.content_hash),
        sync_status: SyncStatus::BaselineUpdated,
    };

    Ok(Json(response))
}

/// Handler for `GET /api/v1/sync/pull`.
pub async fn pull_handler(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Query(query): Query<PullQuery>,
) -> Result<Json<SyncPullResponse>, ServerError> {
    let branch = query.branch.unwrap_or_else(|| "main".to_string());

    let snapshot_opt = state
        .db
        .get_latest_snapshot(&query.repo_id, &branch)
        .await?;

    let Some((snapshot_row, finding_rows)) = snapshot_opt else {
        return Ok(Json(SyncPullResponse {
            repo_id: query.repo_id,
            branch,
            baseline_commit_hash: None,
            baseline_health_score: None,
            baseline_findings: Vec::new(),
            baseline_content_hash: None,
            sync_status: SyncStatus::NotFound,
        }));
    };

    let health_score: HealthScore = serde_json::from_str(&snapshot_row.health_json)?;

    let mut baseline_findings = Vec::with_capacity(finding_rows.len());
    for row in finding_rows {
        if let Ok(f) = serde_json::from_str::<Finding>(&row.finding_json) {
            baseline_findings.push(f);
        }
    }

    Ok(Json(SyncPullResponse {
        repo_id: query.repo_id,
        branch,
        baseline_commit_hash: Some(snapshot_row.commit_hash),
        baseline_health_score: Some(health_score),
        baseline_findings,
        baseline_content_hash: Some(snapshot_row.content_hash),
        sync_status: SyncStatus::UpToDate,
    }))
}
