//! `forge-server` — Lightweight self-hostable synchronization backend for ELCODE Forge.

pub mod app;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod history;
pub mod projects;
pub mod routes;
pub mod state;
pub mod webhooks;

pub use app::{create_app, run_server, shutdown_signal};
pub use auth::{hash_api_key, issue_jwt, verify_jwt, AuthenticatedUser, Claims};
pub use config::ServerConfig;
pub use db::Db;
pub use error::ServerError;
pub use history::{calculate_history_trend, HistoryTrend, TrendDirection};
pub use projects::{get_project_overview, ProjectOverview};
pub use state::AppState;
pub use webhooks::{
    dispatch_webhooks, evaluate_and_dispatch_events, format_discord_payload, format_slack_payload,
    generate_hmac_signature, verify_hmac_signature, EventEvaluationContext, WebhookEvent,
    WebhookPayload,
};

/// Returns current version string for `forge-server`.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
