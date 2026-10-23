//! JWT token generation, verification, and API key hashing.

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;

/// Standard JWT payload claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject identifier (user, team, or token ID).
    pub sub: String,
    /// Authorization role (e.g., "admin", "agent", "ci").
    pub role: String,
    /// Expiration timestamp in seconds since Unix epoch.
    pub exp: usize,
    /// Issued-at timestamp in seconds since Unix epoch.
    pub iat: usize,
}

/// Issues a new signed JWT token.
pub fn issue_jwt(
    secret: &str,
    sub: &str,
    role: &str,
    valid_secs: u64,
) -> Result<String, ServerError> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: sub.to_string(),
        role: role.to_string(),
        exp: now + valid_secs as usize,
        iat: now,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ServerError::Auth(format!("Failed to sign JWT: {e}")))?;

    Ok(token)
}

/// Validates and decodes a signed JWT token.
pub fn verify_jwt(secret: &str, token: &str) -> Result<Claims, ServerError> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| ServerError::Auth(format!("Invalid JWT token: {e}")))?;

    Ok(data.claims)
}

/// Computes deterministic BLAKE3 lowercase hexadecimal hash of an API key for safe storage.
pub fn hash_api_key(key: &str) -> String {
    let hash = blake3::hash(key.as_bytes());
    hash.to_hex().to_string()
}
