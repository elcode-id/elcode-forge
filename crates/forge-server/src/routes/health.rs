//! Health check endpoint.

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::state::AppState;

/// Health check response body.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub database: &'static str,
}

/// Handler for `GET /api/v1/health`.
pub async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        database: state.db.backend_name(),
    })
}
