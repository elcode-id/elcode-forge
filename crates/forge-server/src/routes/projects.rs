//! Project workspace registry endpoints.

use axum::extract::{Path, State};
use axum::Json;

use crate::auth::AuthenticatedUser;
use crate::db::models::{BranchSummary, ProjectSummary};
use crate::error::ServerError;
use crate::projects::{get_project_overview, ProjectOverview};
use crate::state::AppState;

/// Handler for `GET /api/v1/projects`.
pub async fn list_projects_handler(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<Vec<ProjectSummary>>, ServerError> {
    let projects = state.db.list_projects().await?;
    Ok(Json(projects))
}

/// Handler for `GET /api/v1/projects/:repo_id/overview`.
pub async fn project_overview_handler(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(repo_id): Path<String>,
) -> Result<Json<ProjectOverview>, ServerError> {
    let overview_opt = get_project_overview(&state.db, &repo_id).await?;
    match overview_opt {
        Some(overview) => Ok(Json(overview)),
        None => Err(ServerError::NotFound(format!(
            "Project '{repo_id}' not found"
        ))),
    }
}

/// Handler for `GET /api/v1/projects/:repo_id/branches`.
pub async fn project_branches_handler(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(repo_id): Path<String>,
) -> Result<Json<Vec<BranchSummary>>, ServerError> {
    let branches = state.db.get_project_branches(&repo_id).await?;
    Ok(Json(branches))
}
