use std::fs::{create_dir_all, read_to_string, write};
use std::path::{Path, PathBuf};

use forge_core::error::ForgeError;

/// File-based cache for AI responses stored under `<repo_root>/.eforge/ai-cache/<blake3_hash>.json`.
///
/// Ensures deterministic caching of structured outputs and avoids redundant calls to LLM providers.
pub struct ResponseCache {
    cache_dir: PathBuf,
}

impl ResponseCache {
    /// Creates a new `ResponseCache` instance rooted at `<repo_root>/.eforge/ai-cache`.
    #[must_use]
    pub fn new(repo_root: &Path) -> Self {
        Self {
            cache_dir: repo_root.join(".eforge").join("ai-cache"),
        }
    }

    /// Computes a deterministic 64-character lowercase hex BLAKE3 cache key.
    ///
    /// Key format incorporates provider, model, system prompt, payload, and optional response schema.
    #[must_use]
    pub fn key(
        provider: &str,
        model: &str,
        system: &str,
        payload: &str,
        schema: Option<&str>,
    ) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(provider.as_bytes());
        hasher.update(b"\0");
        hasher.update(model.as_bytes());
        hasher.update(b"\0");
        hasher.update(system.as_bytes());
        hasher.update(b"\0");
        hasher.update(payload.as_bytes());
        hasher.update(b"\0");
        hasher.update(schema.unwrap_or("").as_bytes());
        hasher.finalize().to_hex().to_string()
    }

    /// Validates that a cache key is strictly a 64-character lowercase hexadecimal string.
    ///
    /// This prevents path traversal or directory escape attempts outside the cache directory.
    fn validate_key(key: &str) -> Result<(), ForgeError> {
        if key.len() == 64 && key.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(())
        } else {
            Err(ForgeError::Ai {
                message: format!(
                    "invalid cache key format: expected 64-character hex BLAKE3 digest, got '{key}'"
                ),
            })
        }
    }

    /// Retrieves a cached JSON response if it exists.
    ///
    /// Returns `Ok(None)` if the key has not been cached yet.
    pub fn get(&self, key: &str) -> Result<Option<serde_json::Value>, ForgeError> {
        Self::validate_key(key)?;
        let path = self.cache_dir.join(format!("{key}.json"));
        if !path.exists() {
            return Ok(None);
        }

        let content = read_to_string(&path).map_err(|source| ForgeError::Io {
            path: path.clone(),
            source,
        })?;

        let val: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| ForgeError::Serialization {
                message: format!(
                    "failed to deserialize cached response from {}: {e}",
                    path.display()
                ),
            })?;

        Ok(Some(val))
    }

    /// Stores a JSON response in the cache under `<key>.json`.
    pub fn put(&self, key: &str, value: &serde_json::Value) -> Result<(), ForgeError> {
        Self::validate_key(key)?;
        create_dir_all(&self.cache_dir).map_err(|source| ForgeError::Io {
            path: self.cache_dir.clone(),
            source,
        })?;

        let path = self.cache_dir.join(format!("{key}.json"));
        let json_str =
            serde_json::to_string_pretty(value).map_err(|e| ForgeError::Serialization {
                message: format!("failed to serialize cache value for {key}: {e}"),
            })?;

        write(&path, json_str).map_err(|source| ForgeError::Io { path, source })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_is_deterministic_64_char_hex() {
        let k1 = ResponseCache::key("openai", "gpt-4o", "sys", "payload", Some("schema"));
        let k2 = ResponseCache::key("openai", "gpt-4o", "sys", "payload", Some("schema"));
        let k3 = ResponseCache::key("anthropic", "claude-3-5", "sys", "payload", None);

        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
        assert_eq!(k1.len(), 64);
        assert!(k1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn response_cache_put_get_roundtrip_and_missing() {
        let temp_dir = std::env::temp_dir().join(format!(
            "eforge_cache_test_{}",
            blake3::hash(b"roundtrip_test").to_hex()
        ));
        let cache = ResponseCache::new(&temp_dir);
        let key = ResponseCache::key("mock", "m1", "sys", "user", None);

        // Initially miss
        let initial = cache.get(&key);
        assert!(matches!(initial, Ok(None)));

        // Put value
        let val = serde_json::json!({
            "status": "success",
            "score": 95
        });
        assert!(cache.put(&key, &val).is_ok());

        // Get value
        let retrieved = cache.get(&key);
        assert!(matches!(retrieved, Ok(Some(ref v)) if v == &val));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn response_cache_rejects_invalid_key_traversal() {
        let temp_dir = std::env::temp_dir().join("eforge_cache_invalid_key_test");
        let cache = ResponseCache::new(&temp_dir);

        let traversal_key = "../../../etc/passwd";
        let short_key = "abc123";

        assert!(cache.get(traversal_key).is_err());
        assert!(cache.get(short_key).is_err());
        assert!(cache.put(traversal_key, &serde_json::json!({})).is_err());
    }
}
