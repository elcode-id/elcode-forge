//! JSON Web Key Set (JWKS) key caching and verification.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::auth::jwt::Claims;
use crate::error::ServerError;

/// Individual JSON Web Key descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwkKey {
    pub kty: String,
    #[serde(default)]
    pub kid: Option<String>,
    #[serde(default)]
    pub alg: Option<String>,
    #[serde(default)]
    pub n: Option<String>,
    #[serde(default)]
    pub e: Option<String>,
    #[serde(default)]
    pub k: Option<String>,
}

/// Document format returned by `/.well-known/jwks.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JwksDocument {
    pub keys: Vec<JwkKey>,
}

/// Thread-safe in-memory cache for JWKS keys with TTL.
#[derive(Clone, Default)]
pub struct JwksCache {
    keys: Arc<RwLock<BTreeMap<String, JwkKey>>>,
    last_fetched: Arc<RwLock<Option<Instant>>>,
    ttl: Duration,
}

impl JwksCache {
    /// Creates a new JWKS cache with a 1-hour TTL.
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(BTreeMap::new())),
            last_fetched: Arc::new(RwLock::new(None)),
            ttl: Duration::from_secs(3600),
        }
    }

    /// Loads static keys from a `JwksDocument` (useful for testing or air-gapped deployments).
    pub async fn load_static_keys(&self, doc: JwksDocument) {
        let mut lock = self.keys.write().await;
        for key in doc.keys {
            if let Some(kid) = &key.kid {
                lock.insert(kid.clone(), key);
            }
        }
        let mut last = self.last_fetched.write().await;
        *last = Some(Instant::now());
    }

    /// Refreshes keys from the configured remote JWKS URL if expired or missing.
    pub async fn ensure_fresh_keys(&self, url: &str) -> Result<(), ServerError> {
        let needs_refresh = {
            let last = self.last_fetched.read().await;
            match *last {
                Some(inst) => inst.elapsed() > self.ttl,
                None => true,
            }
        };

        if needs_refresh {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .map_err(|e| ServerError::Internal(format!("Failed to build HTTP client: {e}")))?;

            let response = client.get(url).send().await.map_err(|e| {
                ServerError::Auth(format!("Failed to fetch JWKS from '{url}': {e}"))
            })?;

            let doc: JwksDocument = response.json().await.map_err(|e| {
                ServerError::Auth(format!("Failed to parse JWKS JSON response: {e}"))
            })?;

            self.load_static_keys(doc).await;
        }

        Ok(())
    }

    /// Attempts to verify and decode a token using keys stored in the JWKS cache.
    pub async fn verify_token(
        &self,
        token: &str,
        jwks_url: Option<&str>,
    ) -> Result<Claims, ServerError> {
        let header = decode_header(token)
            .map_err(|e| ServerError::Auth(format!("Invalid token header: {e}")))?;

        let kid = header
            .kid
            .ok_or_else(|| ServerError::Auth("Token header missing 'kid'".to_string()))?;

        // If key not in cache and url is provided, refresh keys
        {
            let lock = self.keys.read().await;
            if !lock.contains_key(&kid) {
                drop(lock);
                if let Some(url) = jwks_url {
                    let _ = self.ensure_fresh_keys(url).await;
                }
            }
        }

        let lock = self.keys.read().await;
        let jwk = lock
            .get(&kid)
            .ok_or_else(|| ServerError::Auth(format!("Unknown key ID '{kid}' in JWKS cache")))?;

        let decoding_key = if let (Some(n), Some(e)) = (&jwk.n, &jwk.e) {
            DecodingKey::from_rsa_components(n, e).map_err(|e| {
                ServerError::Auth(format!("Invalid RSA key components in JWKS: {e}"))
            })?
        } else if let Some(k) = &jwk.k {
            DecodingKey::from_secret(k.as_bytes())
        } else {
            return Err(ServerError::Auth(
                "Unsupported key format in JWKS key set".to_string(),
            ));
        };

        let mut validation = Validation::new(header.alg);
        if header.alg == Algorithm::RS256
            || header.alg == Algorithm::RS384
            || header.alg == Algorithm::RS512
        {
            validation.validate_exp = true;
        }

        let data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| ServerError::Auth(format!("JWKS signature verification failed: {e}")))?;

        Ok(data.claims)
    }
}
