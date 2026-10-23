//! `forge-server` — Lightweight self-hostable synchronization backend for ELCODE Forge.

pub mod app;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod routes;
pub mod state;

pub use app::{create_app, run_server, shutdown_signal};
pub use auth::{hash_api_key, issue_jwt, verify_jwt, AuthenticatedUser, Claims};
pub use config::ServerConfig;
pub use db::Db;
pub use error::ServerError;
pub use state::AppState;

/// Returns current version string for `forge-server`.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
