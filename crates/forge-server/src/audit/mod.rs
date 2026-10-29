//! Append-only audit event logging and recorder.

use crate::db::models::AuditEventRow;
use crate::db::Db;
use crate::error::ServerError;

/// Standard audit event action constants.
pub mod actions {
    pub const SNAPSHOT_PUSH: &str = "snapshot.push";
    pub const ORG_CREATE: &str = "org.create";
    pub const PROJECT_LINK: &str = "project.link";
    pub const WEBHOOK_CREATE: &str = "webhook.create";
    pub const WEBHOOK_DELETE: &str = "webhook.delete";
    pub const API_KEY_CREATE: &str = "api_key.create";
}

/// Standard audit resource types.
pub mod resources {
    pub const SNAPSHOT: &str = "snapshot";
    pub const ORGANIZATION: &str = "organization";
    pub const PROJECT: &str = "project";
    pub const WEBHOOK: &str = "webhook";
    pub const API_KEY: &str = "api_key";
}

/// Parameters for creating and persisting an audit event.
#[derive(Debug, Clone)]
pub struct AuditEventParams<'a> {
    pub org_id: &'a str,
    pub project_id: Option<&'a str>,
    pub actor: &'a str,
    pub action: &'a str,
    pub resource_type: &'a str,
    pub resource_id: &'a str,
    pub details: Option<&'a serde_json::Value>,
}

/// Records an append-only audit event.
pub async fn record_audit_event(
    db: &Db,
    params: AuditEventParams<'_>,
) -> Result<AuditEventRow, ServerError> {
    let now = chrono::Utc::now().timestamp();
    let nanos = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or(now * 1_000_000_000);
    let id_hash = blake3::hash(
        format!(
            "{}:{}:{}:{}",
            params.org_id, params.actor, params.action, nanos
        )
        .as_bytes(),
    )
    .to_hex()
    .to_string();
    let event_id = format!("evt_{}", &id_hash[..16]);

    let details_json = match params.details {
        Some(v) => Some(serde_json::to_string(v)?),
        None => None,
    };

    let row = AuditEventRow {
        id: event_id,
        org_id: params.org_id.to_string(),
        project_id: params.project_id.map(|s| s.to_string()),
        actor: params.actor.to_string(),
        action: params.action.to_string(),
        resource_type: params.resource_type.to_string(),
        resource_id: params.resource_id.to_string(),
        details_json,
        created_at: now,
    };

    db.record_audit_event(&row).await?;
    Ok(row)
}
