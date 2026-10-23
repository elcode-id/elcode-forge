//! Shared application state.

use std::sync::Arc;

use crate::config::ServerConfig;
use crate::db::Db;

/// Shared application state across HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub config: Arc<ServerConfig>,
}

impl AppState {
    pub fn new(db: Db, config: ServerConfig) -> Self {
        Self {
            db: Arc::new(db),
            config: Arc::new(config),
        }
    }
}
