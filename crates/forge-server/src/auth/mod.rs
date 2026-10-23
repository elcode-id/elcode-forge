//! Authentication layer providing API key and JWT bearer verification.

pub mod jwt;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

use crate::error::ServerError;
use crate::state::AppState;
pub use jwt::{hash_api_key, issue_jwt, verify_jwt, Claims};

/// Information extracted from an authorized request.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// Subject identifier (e.g. user, token name, or "admin").
    pub subject: String,
    /// Role identifier (e.g. "admin", "agent", "ci").
    pub role: String,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ServerError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        let Some(header_val) = auth_header else {
            return Err(ServerError::Auth(
                "Missing Authorization header. Use 'Authorization: Bearer <token>'".to_string(),
            ));
        };

        let token = if let Some(stripped) = header_val.strip_prefix("Bearer ") {
            stripped.trim()
        } else {
            return Err(ServerError::Auth(
                "Malformed Authorization header. Format must be 'Bearer <token>'".to_string(),
            ));
        };

        if token.is_empty() {
            return Err(ServerError::Auth(
                "Authentication token cannot be empty".to_string(),
            ));
        }

        // 1. Direct admin API key match
        if let Some(admin_key) = &state.config.admin_api_key {
            if token == admin_key {
                return Ok(AuthenticatedUser {
                    subject: "admin".to_string(),
                    role: "admin".to_string(),
                });
            }
        }

        // 2. Database API key lookup by BLAKE3 hash
        let key_hash = hash_api_key(token);
        let now = chrono::Utc::now().timestamp();
        if let Ok(true) = state.db.validate_api_key_hash(&key_hash, now).await {
            return Ok(AuthenticatedUser {
                subject: format!("api_key:{}", &key_hash[..8]),
                role: "api_client".to_string(),
            });
        }

        // 3. JWT verification
        if let Ok(claims) = verify_jwt(&state.config.jwt_secret, token) {
            return Ok(AuthenticatedUser {
                subject: claims.sub,
                role: claims.role,
            });
        }

        Err(ServerError::Auth(
            "Invalid or expired authentication credentials".to_string(),
        ))
    }
}
