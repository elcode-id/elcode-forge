//! Organization management and multi-repository rollup endpoints.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::AuthenticatedUser;
use crate::db::models::{CrossRepoVulnerability, OrganizationRow, OrganizationSummary};
use crate::error::ServerError;
use crate::org::{get_organization_summary, recompute_org_rollup};
use crate::state::AppState;

/// Request payload to register a new organization.
#[derive(Debug, Deserialize)]
pub struct CreateOrgRequest {
    pub name: String,
    pub slug: Option<String>,
}

/// Request payload to link a project repository to an organization.
#[derive(Debug, Deserialize)]
pub struct LinkProjectRequest {
    pub project_id: String,
}

/// Lightweight item representation for organization listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgListItem {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub health_score: u8,
    pub total_projects: usize,
    pub total_vulnerabilities: usize,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Handler for `POST /api/v1/orgs`.
pub async fn create_org_handler(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<CreateOrgRequest>,
) -> Result<(StatusCode, Json<OrganizationRow>), ServerError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(ServerError::BadRequest(
            "Organization name cannot be empty".to_string(),
        ));
    }

    let slug = payload
        .slug
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            name.to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect()
        });

    if state.db.get_organization(&slug).await?.is_some() {
        return Err(ServerError::BadRequest(format!(
            "Organization with slug '{slug}' already exists"
        )));
    }

    let now = chrono::Utc::now().timestamp();
    let nanos = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or(now * 1_000_000_000);
    let id_hash = blake3::hash(format!("{}:{}:{}", slug, now, nanos).as_bytes())
        .to_hex()
        .to_string();
    let org_id = format!("org_{}", &id_hash[..16]);

    let org_row = OrganizationRow {
        id: org_id.clone(),
        name: name.to_string(),
        slug: slug.clone(),
        created_at: now,
    };

    state.db.create_organization(&org_row).await?;

    // Seed empty materialized rollup
    let _ = recompute_org_rollup(&state.db, &org_id).await?;

    // Record audit event
    let _ = crate::audit::record_audit_event(
        &state.db,
        crate::audit::AuditEventParams {
            org_id: &org_id,
            project_id: None,
            actor: &user.subject,
            action: crate::audit::actions::ORG_CREATE,
            resource_type: crate::audit::resources::ORGANIZATION,
            resource_id: &org_id,
            details: Some(&serde_json::json!({ "name": name, "slug": slug })),
        },
    )
    .await;

    Ok((StatusCode::CREATED, Json(org_row)))
}

/// Handler for `GET /api/v1/orgs`.
pub async fn list_orgs_handler(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<Vec<OrgListItem>>, ServerError> {
    let orgs = state.db.list_organizations().await?;
    let mut list = Vec::with_capacity(orgs.len());

    for org in orgs {
        let (health_score, total_projects, total_vulnerabilities, updated_at) =
            if let Some(rollup) = state.db.get_organization_rollup(&org.id).await? {
                (
                    rollup.health_score as u8,
                    rollup.total_projects as usize,
                    rollup.total_vulnerabilities as usize,
                    rollup.updated_at,
                )
            } else {
                (100, 0, 0, org.created_at)
            };

        list.push(OrgListItem {
            id: org.id,
            name: org.name,
            slug: org.slug,
            health_score,
            total_projects,
            total_vulnerabilities,
            created_at: org.created_at,
            updated_at,
        });
    }

    Ok(Json(list))
}

/// Handler for `POST /api/v1/orgs/:id/projects`.
pub async fn link_project_handler(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    Json(payload): Json<LinkProjectRequest>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let org = state
        .db
        .get_organization(&id)
        .await?
        .ok_or_else(|| ServerError::NotFound(format!("Organization '{id}' not found")))?;

    let project_id = payload.project_id.trim();
    if project_id.is_empty() {
        return Err(ServerError::BadRequest(
            "project_id cannot be empty".to_string(),
        ));
    }

    let now = chrono::Utc::now().timestamp();
    // Ensure project exists in projects table so foreign keys hold
    if state.db.get_project(project_id).await?.is_none() {
        state.db.upsert_project(project_id, project_id, now).await?;
    }

    state
        .db
        .link_project_to_organization(&org.id, project_id, now)
        .await?;

    // Recompute rollup materialized view
    recompute_org_rollup(&state.db, &org.id).await?;

    // Record audit event
    let _ = crate::audit::record_audit_event(
        &state.db,
        crate::audit::AuditEventParams {
            org_id: &org.id,
            project_id: Some(project_id),
            actor: &user.subject,
            action: crate::audit::actions::PROJECT_LINK,
            resource_type: crate::audit::resources::PROJECT,
            resource_id: project_id,
            details: None,
        },
    )
    .await;

    Ok(Json(serde_json::json!({
        "status": "linked",
        "org_id": org.id,
        "project_id": project_id
    })))
}

/// Handler for `GET /api/v1/orgs/:id/summary`.
pub async fn get_org_summary_handler(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<Json<OrganizationSummary>, ServerError> {
    let summary = get_organization_summary(&state.db, &id).await?;
    Ok(Json(summary))
}

/// Handler for `GET /api/v1/orgs/:id/vulnerabilities`.
pub async fn get_org_vulnerabilities_handler(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<Json<Vec<CrossRepoVulnerability>>, ServerError> {
    let summary = get_organization_summary(&state.db, &id).await?;
    Ok(Json(summary.cross_repo_vulnerabilities))
}
