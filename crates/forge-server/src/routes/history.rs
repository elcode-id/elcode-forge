//! Multi-branch historical score retention and trend endpoints.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::auth::AuthenticatedUser;
use crate::error::ServerError;
use crate::history::{calculate_history_trend, HistoryTrend};
use crate::state::AppState;

/// Query parameters for history retrieval.
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub branch: Option<String>,
    pub limit: Option<i64>,
}

/// Handler for `GET /api/v1/projects/:repo_id/history`.
pub async fn project_history_handler(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(repo_id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<HistoryTrend>, ServerError> {
    let branch = query.branch.unwrap_or_else(|| "main".to_string());
    let limit = query.limit.unwrap_or(50).clamp(1, 500);

    let points = state
        .db
        .get_branch_history(&repo_id, &branch, limit)
        .await?;
    let trend = calculate_history_trend(&repo_id, &branch, points);

    Ok(Json(trend))
}
