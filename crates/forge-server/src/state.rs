//! Shared application state.

use std::sync::Arc;

use crate::auth::jwks::JwksCache;
use crate::config::ServerConfig;
use crate::db::Db;

/// Shared application state across HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub config: Arc<ServerConfig>,
    pub jwks: Arc<JwksCache>,
}

impl AppState {
    pub fn new(db: Db, config: ServerConfig) -> Self {
        Self {
            db: Arc::new(db),
            config: Arc::new(config),
            jwks: Arc::new(JwksCache::new()),
        }
    }
}
