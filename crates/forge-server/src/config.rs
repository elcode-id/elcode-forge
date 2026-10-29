//! Server configuration loader for `forge-server`.

use std::env;

use crate::error::ServerError;

/// Runtime server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Bind address host (default: `127.0.0.1`).
    pub host: String,
    /// Bind address port (default: `8080`).
    pub port: u16,
    /// Connection string for SQLite or PostgreSQL.
    pub database_url: String,
    /// Secret key used for signing and verifying JWT tokens.
    pub jwt_secret: String,
    /// Optional predefined admin API key bootstrapped at startup.
    pub admin_api_key: Option<String>,
    /// Token expiration duration in seconds (default: 7 days).
    pub jwt_expiration_secs: u64,
    /// Whether to trust reverse-proxy identity headers (X-Forwarded-User, X-Forwarded-Role, X-Forwarded-Email).
    pub trust_proxy_headers: bool,
    /// Optional JWKS endpoint URL for validating external JWTs.
    pub jwks_url: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            database_url: "sqlite://forge.db?mode=rwc".to_string(),
            jwt_secret: "forge-dev-secret-key-must-be-replaced-in-production-32-chars".to_string(),
            admin_api_key: None,
            jwt_expiration_secs: 7 * 24 * 3600,
            trust_proxy_headers: false,
            jwks_url: None,
        }
    }
}

impl ServerConfig {
    /// Loads configuration from standard environment variables with fallbacks.
    pub fn from_env() -> Result<Self, ServerError> {
        let mut config = Self::default();

        if let Ok(host) = env::var("EFORGE_SERVER_HOST") {
            config.host = host;
        }

        if let Ok(port_str) = env::var("EFORGE_SERVER_PORT") {
            match port_str.parse::<u16>() {
                Ok(p) => config.port = p,
                Err(_) => {
                    return Err(ServerError::BadRequest(format!(
                        "Invalid EFORGE_SERVER_PORT '{port_str}'"
                    )));
                }
            }
        }

        if let Ok(db_url) = env::var("EFORGE_DATABASE_URL") {
            config.database_url = db_url;
        } else if let Ok(db_url) = env::var("DATABASE_URL") {
            config.database_url = db_url;
        }

        if let Ok(secret) = env::var("EFORGE_JWT_SECRET") {
            config.jwt_secret = secret;
        }

        if let Ok(admin_key) = env::var("EFORGE_ADMIN_API_KEY") {
            config.admin_api_key = Some(admin_key);
        }

        if let Ok(exp_str) = env::var("EFORGE_JWT_EXPIRATION_SECS") {
            if let Ok(exp) = exp_str.parse::<u64>() {
                config.jwt_expiration_secs = exp;
            }
        }

        if let Ok(val) = env::var("EFORGE_TRUST_PROXY_HEADERS") {
            config.trust_proxy_headers = val.eq_ignore_ascii_case("true") || val == "1";
        }

        if let Ok(url) = env::var("EFORGE_JWKS_URL") {
            if !url.trim().is_empty() {
                config.jwks_url = Some(url);
            }
        }

        Ok(config)
    }

    /// Formats socket bind address `host:port`.
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
