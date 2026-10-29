//! Authentication layer providing API key, JWT bearer, JWKS, and reverse-proxy identity verification.

pub mod jwks;
pub mod jwt;
pub mod rbac;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

pub use jwks::{JwkKey, JwksCache, JwksDocument};
pub use jwt::{hash_api_key, issue_jwt, verify_jwt, Claims};
pub use rbac::Role;

use crate::error::ServerError;
use crate::state::AppState;

/// Information extracted from an authorized request.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// Subject identifier (e.g. user, token name, or "admin").
    pub subject: String,
    /// Authorization role with privilege checks.
    pub role: Role,
    /// Optional user email injected from reverse proxy or token claims.
    pub email: Option<String>,
}

impl AuthenticatedUser {
    /// Whether user holds Admin role.
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = ServerError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 1. Reverse-proxy identity header parsing (only active when explicitly enabled)
        if state.config.trust_proxy_headers {
            if let Some(user_hdr) = parts.headers.get("x-forwarded-user") {
                if let Ok(username) = user_hdr.to_str() {
                    let trimmed = username.trim();
                    if !trimmed.is_empty() {
                        let role = parts
                            .headers
                            .get("x-forwarded-role")
                            .and_then(|h| h.to_str().ok())
                            .map(Role::from_str_loose)
                            .unwrap_or(Role::Maintainer);

                        let email = parts
                            .headers
                            .get("x-forwarded-email")
                            .and_then(|h| h.to_str().ok())
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty());

                        return Ok(AuthenticatedUser {
                            subject: trimmed.to_string(),
                            role,
                            email,
                        });
                    }
                }
            }
        }

        // 2. Standard Authorization: Bearer <token>
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

        // 3. Direct admin API key match
        if let Some(admin_key) = &state.config.admin_api_key {
            if token == admin_key {
                return Ok(AuthenticatedUser {
                    subject: "admin".to_string(),
                    role: Role::Admin,
                    email: None,
                });
            }
        }

        // 4. Database API key lookup by BLAKE3 hash
        let key_hash = hash_api_key(token);
        let now = chrono::Utc::now().timestamp();
        if let Ok(true) = state.db.validate_api_key_hash(&key_hash, now).await {
            return Ok(AuthenticatedUser {
                subject: format!("api_key:{}", &key_hash[..8]),
                role: Role::Maintainer,
                email: None,
            });
        }

        // 5. JWKS remote/cached key verification
        if state.config.jwks_url.is_some() {
            if let Ok(claims) = state
                .jwks
                .verify_token(token, state.config.jwks_url.as_deref())
                .await
            {
                return Ok(AuthenticatedUser {
                    subject: claims.sub,
                    role: Role::from_str_loose(&claims.role),
                    email: None,
                });
            }
        }

        // 6. Local symmetric JWT verification
        if let Ok(claims) = verify_jwt(&state.config.jwt_secret, token) {
            return Ok(AuthenticatedUser {
                subject: claims.sub,
                role: Role::from_str_loose(&claims.role),
                email: None,
            });
        }

        Err(ServerError::Auth(
            "Invalid or expired authentication credentials".to_string(),
        ))
    }
}
