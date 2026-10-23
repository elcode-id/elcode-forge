//! Authentication and token issuance endpoints.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::jwt::{hash_api_key, issue_jwt};
use crate::error::ServerError;
use crate::state::AppState;

/// Request payload for token issuance.
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    /// API key provided for exchange into a signed session JWT.
    pub api_key: Option<String>,
    /// Optional username / subject identifier.
    pub username: Option<String>,
}

/// Response payload containing issued JWT.
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
}

/// Handler for `POST /api/v1/auth/token`.
pub async fn token_handler(
    State(state): State<AppState>,
    Json(payload): Json<TokenRequest>,
) -> Result<Json<TokenResponse>, ServerError> {
    let raw_key = match payload.api_key {
        Some(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => {
            return Err(ServerError::BadRequest(
                "Missing 'api_key' in request body".to_string(),
            ));
        }
    };

    let now = chrono::Utc::now().timestamp();
    let mut authenticated = false;
    let mut subject = payload.username.unwrap_or_else(|| "api_user".to_string());
    let mut role = "client";

    // 1. Check against server admin API key
    if let Some(admin_key) = &state.config.admin_api_key {
        if &raw_key == admin_key {
            authenticated = true;
            subject = "admin".to_string();
            role = "admin";
        }
    }

    // 2. Check against database stored API keys
    if !authenticated {
        let key_hash = hash_api_key(&raw_key);
        if state.db.validate_api_key_hash(&key_hash, now).await? {
            authenticated = true;
            role = "member";
        }
    }

    if !authenticated {
        return Err(ServerError::Auth("Invalid API key".to_string()));
    }

    let token = issue_jwt(
        &state.config.jwt_secret,
        &subject,
        role,
        state.config.jwt_expiration_secs,
    )?;

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "Bearer",
        expires_in: state.config.jwt_expiration_secs,
    }))
}
