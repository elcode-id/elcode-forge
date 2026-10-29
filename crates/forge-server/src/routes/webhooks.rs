//! Outbound webhook subscription endpoints.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::AuthenticatedUser;
use crate::db::models::WebhookRow;
use crate::error::ServerError;
use crate::state::AppState;

/// Request payload to register a new outbound webhook.
#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub repo_id: Option<String>,
    pub url: String,
    pub secret: Option<String>,
    pub events: Option<Vec<String>>,
    pub min_health_threshold: Option<i64>,
}

/// Query parameters for listing webhooks.
#[derive(Debug, Deserialize)]
pub struct ListWebhooksQuery {
    pub repo_id: Option<String>,
}

/// Masked webhook summary for secure API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSummaryResponse {
    pub id: String,
    pub repo_id: Option<String>,
    pub url: String,
    pub has_secret: bool,
    pub events: Vec<String>,
    pub min_health_threshold: Option<i64>,
    pub is_active: bool,
    pub created_at: i64,
}

impl From<WebhookRow> for WebhookSummaryResponse {
    fn from(row: WebhookRow) -> Self {
        let events: Vec<String> = serde_json::from_str(&row.events_json).unwrap_or_default();
        Self {
            id: row.id,
            repo_id: row.repo_id,
            url: row.url,
            has_secret: row.secret.is_some(),
            events,
            min_health_threshold: row.min_health_threshold,
            is_active: row.is_active == 1,
            created_at: row.created_at,
        }
    }
}

/// Handler for `POST /api/v1/webhooks`.
pub async fn create_webhook_handler(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(payload): Json<CreateWebhookRequest>,
) -> Result<(StatusCode, Json<WebhookSummaryResponse>), ServerError> {
    if payload.url.is_empty()
        || (!payload.url.starts_with("http://") && !payload.url.starts_with("https://"))
    {
        return Err(ServerError::BadRequest(
            "Webhook url must start with http:// or https://".to_string(),
        ));
    }

    let now = chrono::Utc::now().timestamp();
    let nanos = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or(now * 1_000_000_000);
    let id_hash = blake3::hash(format!("{}:{}:{}", payload.url, now, nanos).as_bytes())
        .to_hex()
        .to_string();
    let webhook_id = format!("wh_{}", &id_hash[..16]);

    let events = payload.events.unwrap_or_else(|| vec!["*".to_string()]);
    let events_json = serde_json::to_string(&events)?;

    let row = WebhookRow {
        id: webhook_id,
        repo_id: payload.repo_id,
        url: payload.url,
        secret: payload.secret,
        events_json,
        min_health_threshold: payload.min_health_threshold,
        is_active: 1,
        created_at: now,
    };

    state.db.create_webhook(&row).await?;

    let org_id = row.repo_id.as_deref().unwrap_or("default");
    let _ = crate::audit::record_audit_event(
        &state.db,
        crate::audit::AuditEventParams {
            org_id,
            project_id: row.repo_id.as_deref(),
            actor: &_user.subject,
            action: crate::audit::actions::WEBHOOK_CREATE,
            resource_type: crate::audit::resources::WEBHOOK,
            resource_id: &row.id,
            details: Some(&serde_json::json!({ "url": row.url })),
        },
    )
    .await;

    Ok((StatusCode::CREATED, Json(WebhookSummaryResponse::from(row))))
}

/// Handler for `GET /api/v1/webhooks`.
pub async fn list_webhooks_handler(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Query(query): Query<ListWebhooksQuery>,
) -> Result<Json<Vec<WebhookSummaryResponse>>, ServerError> {
    let rows = state.db.list_webhooks(query.repo_id.as_deref()).await?;
    let summaries: Vec<WebhookSummaryResponse> =
        rows.into_iter().map(WebhookSummaryResponse::from).collect();
    Ok(Json(summaries))
}

/// Handler for `DELETE /api/v1/webhooks/:id`.
pub async fn delete_webhook_handler(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ServerError> {
    let deleted = state.db.delete_webhook(&id).await?;
    if deleted {
        let _ = crate::audit::record_audit_event(
            &state.db,
            crate::audit::AuditEventParams {
                org_id: "default",
                project_id: None,
                actor: &_user.subject,
                action: crate::audit::actions::WEBHOOK_DELETE,
                resource_type: crate::audit::resources::WEBHOOK,
                resource_id: &id,
                details: None,
            },
        )
        .await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ServerError::NotFound(format!("Webhook '{id}' not found")))
    }
}
