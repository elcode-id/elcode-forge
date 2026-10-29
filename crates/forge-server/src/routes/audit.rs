//! Audit event query endpoints.

use axum::extract::{Query, State};
use axum::Json;

use crate::auth::AuthenticatedUser;
use crate::db::models::{AuditEventFilter, AuditEventRow};
use crate::error::ServerError;
use crate::state::AppState;

/// Handler for `GET /api/v1/audit-events`.
/// Guarded by RBAC: requires `Auditor` or `Admin` role.
pub async fn list_audit_events_handler(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(filter): Query<AuditEventFilter>,
) -> Result<Json<Vec<AuditEventRow>>, ServerError> {
    if !user.role.can_read_audit() {
        return Err(ServerError::Forbidden(format!(
            "Role '{}' is not authorized to view audit events. Requires Auditor or Admin role.",
            user.role
        )));
    }

    let events = state.db.query_audit_events(&filter).await?;
    Ok(Json(events))
}
