//! JSON Web Key Set (JWKS) key caching and verification.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::auth::jwt::Claims;
use crate::error::ServerError;

/// Validates whether a remote JWKS URL is safe to fetch (must be HTTPS or loopback HTTP).
pub fn is_safe_jwks_url(raw_url: &str) -> bool {
    if let Ok(parsed) = reqwest::Url::parse(raw_url) {
        match parsed.scheme() {
            "https" => true,
            "http" => {
                if let Some(host) = parsed.host_str() {
                    host == "localhost" || host == "127.0.0.1" || host == "[::1]" || host == "::1"
                } else {
                    false
                }
            }
            _ => false,
        }
    } else {
        false
    }
}

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

/// Thread-safe in-memory cache for JWKS keys with TTL and shared HTTP client.
#[derive(Clone)]
pub struct JwksCache {
    keys: Arc<RwLock<BTreeMap<String, JwkKey>>>,
    last_fetched: Arc<RwLock<Option<Instant>>>,
    ttl: Duration,
    client: reqwest::Client,
}

impl Default for JwksCache {
    fn default() -> Self {
        Self::new()
    }
}

impl JwksCache {
    /// Creates a new JWKS cache with a 1-hour TTL and shared HTTP client.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            keys: Arc::new(RwLock::new(BTreeMap::new())),
            last_fetched: Arc::new(RwLock::new(None)),
            ttl: Duration::from_secs(3600),
            client,
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
        if !is_safe_jwks_url(url) {
            return Err(ServerError::Auth(format!(
                "Insecure JWKS URL '{url}': must use https:// or loopback http:// (localhost/127.0.0.1)"
            )));
        }

        let needs_refresh = {
            let last = self.last_fetched.read().await;
            match *last {
                Some(inst) => inst.elapsed() > self.ttl,
                None => true,
            }
        };

        if needs_refresh {
            let response = self.client.get(url).send().await.map_err(|e| {
                ServerError::Auth(format!("Failed to fetch JWKS from '{url}': {e}"))
            })?;

            let doc: JwksDocument = response.json().await.map_err(|e| {
                ServerError::Auth(format!("Failed to parse JWKS JSON response: {e}"))
            })?;

            self.load_static_keys(doc).await;
        }

        Ok(())
    }

    /// Attempts to verify and decode an asymmetric RSA token using keys stored in the JWKS cache.
    /// Rejects symmetric algorithms (`HS256`/`HS384`/`HS512`) and symmetric `jwk.k` keys.
    pub async fn verify_token(
        &self,
        token: &str,
        jwks_url: Option<&str>,
    ) -> Result<Claims, ServerError> {
        let header = decode_header(token)
            .map_err(|e| ServerError::Auth(format!("Invalid token header: {e}")))?;

        // Enforce asymmetric-only RS256 / RS384 / RS512 algorithms to prevent algorithm confusion attacks
        match header.alg {
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {}
            _ => {
                return Err(ServerError::Auth(format!(
                    "Unsupported JWKS algorithm '{:?}': only asymmetric RS256/RS384/RS512 are allowed",
                    header.alg
                )));
            }
        }

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

        // Reject symmetric keys (jwk.k or non-RSA kty)
        if jwk.k.is_some() || jwk.kty.to_uppercase() != "RSA" {
            return Err(ServerError::Auth(
                "Symmetric or non-RSA keys are rejected in JWKS verification".to_string(),
            ));
        }

        let (Some(n), Some(e)) = (&jwk.n, &jwk.e) else {
            return Err(ServerError::Auth(
                "Missing RSA key components (n, e) in JWKS".to_string(),
            ));
        };

        let decoding_key = DecodingKey::from_rsa_components(n, e)
            .map_err(|e| ServerError::Auth(format!("Invalid RSA key components in JWKS: {e}")))?;

        let mut validation = Validation::new(header.alg);
        validation.validate_exp = true;

        let data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| ServerError::Auth(format!("JWKS signature verification failed: {e}")))?;

        Ok(data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_safe_jwks_url_restrictions() {
        assert!(is_safe_jwks_url("https://auth.example.com/jwks.json"));
        assert!(is_safe_jwks_url(
            "https://identity.corp.internal/.well-known/jwks.json"
        ));
        assert!(is_safe_jwks_url("http://localhost:8080/jwks.json"));
        assert!(is_safe_jwks_url("http://127.0.0.1:9000/jwks.json"));
        assert!(is_safe_jwks_url("http://[::1]:9000/jwks.json"));

        // SSRF blocked
        assert!(!is_safe_jwks_url(
            "http://169.254.169.254/latest/meta-data/"
        ));
        assert!(!is_safe_jwks_url("http://internal-db.corp:5432/jwks"));
        assert!(!is_safe_jwks_url("ftp://auth.example.com/jwks.json"));
        assert!(!is_safe_jwks_url("file:///etc/passwd"));
        assert!(!is_safe_jwks_url("not a url"));
    }
}
