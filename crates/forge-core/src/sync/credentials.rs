use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ForgeError;

/// Individual remote server credential entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ServerCredential {
    /// Remote server URL (e.g., "https://forge.elcode.id").
    pub server_url: String,
    /// API authentication token (e.g., Personal Access Token or Service Account Key).
    pub token: String,
    /// Authenticated username or email handle, if known.
    pub username: Option<String>,
    /// Token expiration timestamp in seconds since Unix epoch, if applicable.
    pub expires_at: Option<u64>,
}

/// Global credentials configuration stored in `~/.eforge/credentials.yaml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CredentialsConfig {
    /// Default remote server URL used when no explicit target is passed.
    pub default_server: Option<String>,
    /// Map of normalized server URLs to stored credentials.
    #[serde(default)]
    pub servers: BTreeMap<String, ServerCredential>,
}

impl CredentialsConfig {
    /// Creates a new empty [`CredentialsConfig`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or updates a server credential.
    pub fn set_credential(&mut self, cred: ServerCredential) {
        if self.default_server.is_none() {
            self.default_server = Some(cred.server_url.clone());
        }
        self.servers.insert(cred.server_url.clone(), cred);
    }

    /// Retrieves the credential for a specific server URL, if present.
    pub fn get_credential(&self, server_url: &str) -> Option<&ServerCredential> {
        self.servers.get(server_url)
    }

    /// Removes a server credential by its server URL.
    pub fn remove_credential(&mut self, server_url: &str) -> Option<ServerCredential> {
        if self.default_server.as_deref() == Some(server_url) {
            self.default_server = None;
        }
        self.servers.remove(server_url)
    }

    /// Resolves token for a given server URL, falling back to default server or single configured server when None.
    pub fn resolve_token_for(&self, server_url: Option<&str>) -> Option<String> {
        if let Some(target) = server_url {
            return self.servers.get(target).map(|cred| cred.token.clone());
        }
        if let Some(ref def) = self.default_server {
            if let Some(cred) = self.servers.get(def) {
                return Some(cred.token.clone());
            }
        }
        if self.servers.len() == 1 {
            if let Some((_, cred)) = self.servers.iter().next() {
                return Some(cred.token.clone());
            }
        }
        None
    }

    /// Serializes configuration to deterministic YAML.
    pub fn to_yaml(&self) -> String {
        let mut out = String::new();
        if let Some(ref default_server) = self.default_server {
            out.push_str(&format!(
                "default_server: \"{}\"\n",
                escape_yaml(default_server)
            ));
        }
        out.push_str("servers:\n");
        for (url, cred) in &self.servers {
            out.push_str(&format!("  \"{}\":\n", escape_yaml(url)));
            out.push_str(&format!(
                "    server_url: \"{}\"\n",
                escape_yaml(&cred.server_url)
            ));
            out.push_str(&format!("    token: \"{}\"\n", escape_yaml(&cred.token)));
            if let Some(ref username) = cred.username {
                out.push_str(&format!("    username: \"{}\"\n", escape_yaml(username)));
            }
            if let Some(expires_at) = cred.expires_at {
                out.push_str(&format!("    expires_at: {}\n", expires_at));
            }
        }
        out
    }

    /// Parses YAML string into [`CredentialsConfig`].
    pub fn parse_yaml(content: &str) -> Result<Self, ForgeError> {
        let trimmed = content.trim();
        if trimmed.starts_with('{') {
            return serde_json::from_str(trimmed).map_err(|e| ForgeError::Serialization {
                message: format!("failed to parse JSON credentials: {e}"),
            });
        }

        let mut config = CredentialsConfig::default();
        let mut in_servers = false;
        let mut current_server_url: Option<String> = None;
        let mut current_cred: Option<ServerCredential> = None;

        for (idx, raw_line) in content.lines().enumerate() {
            let line_no = idx + 1;
            let without_comment = match raw_line.find('#') {
                Some(pos) => &raw_line[..pos],
                None => raw_line,
            };
            let line_trimmed = without_comment.trim();
            if line_trimmed.is_empty() || line_trimmed == "---" {
                continue;
            }

            let indent = without_comment
                .chars()
                .take_while(|c| c.is_whitespace())
                .count();

            let (raw_key, raw_val) = if let Some(stripped) = line_trimmed.strip_suffix(':') {
                (stripped, "")
            } else {
                match line_trimmed.split_once(':') {
                    Some((k, v)) => (k, v),
                    None => {
                        return Err(ForgeError::Config {
                            message: format!("line {line_no}: expected 'key: value' syntax"),
                        });
                    }
                }
            };

            let key = unquote(raw_key.trim());
            let val = unquote(raw_val.trim());

            if indent == 0 {
                if let Some(cred) = current_cred.take() {
                    let key = current_server_url
                        .take()
                        .unwrap_or_else(|| cred.server_url.clone());
                    config.servers.insert(key, cred);
                }

                if key == "default_server" {
                    in_servers = false;
                    config.default_server = Some(val.to_string());
                } else if key == "servers" {
                    in_servers = true;
                }
            } else if in_servers && indent == 2 {
                if let Some(cred) = current_cred.take() {
                    let key = current_server_url
                        .take()
                        .unwrap_or_else(|| cred.server_url.clone());
                    config.servers.insert(key, cred);
                }

                current_server_url = Some(key.to_string());
                current_cred = Some(ServerCredential {
                    server_url: key.to_string(),
                    token: String::new(),
                    username: None,
                    expires_at: None,
                });
            } else if in_servers && indent >= 4 {
                if let Some(ref mut cred) = current_cred {
                    match key {
                        "server_url" => cred.server_url = val.to_string(),
                        "token" => cred.token = val.to_string(),
                        "username" => cred.username = Some(val.to_string()),
                        "expires_at" => {
                            let parsed = val.parse::<u64>().map_err(|e| ForgeError::Config {
                                message: format!(
                                    "line {line_no}: invalid expires_at timestamp: {e}"
                                ),
                            })?;
                            cred.expires_at = Some(parsed);
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(cred) = current_cred.take() {
            let key = current_server_url
                .take()
                .unwrap_or_else(|| cred.server_url.clone());
            config.servers.insert(key, cred);
        }

        Ok(config)
    }

    /// Serializes configuration to pretty JSON.
    pub fn to_json(&self) -> Result<String, ForgeError> {
        serde_json::to_string_pretty(self).map_err(|e| ForgeError::Serialization {
            message: format!("failed to serialize credentials: {e}"),
        })
    }

    /// Parses configuration from JSON.
    pub fn parse_json(content: &str) -> Result<Self, ForgeError> {
        serde_json::from_str(content).map_err(|e| ForgeError::Serialization {
            message: format!("failed to deserialize credentials JSON: {e}"),
        })
    }
}

/// Resolves user home directory path across operating systems.
pub fn home_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("EFORGE_HOME") {
        return Some(PathBuf::from(path));
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Default credentials file path (`~/.eforge/credentials.yaml`).
pub fn default_credentials_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("EFORGE_CREDENTIALS_PATH") {
        return Some(PathBuf::from(p));
    }
    home_dir().map(|h| h.join(".eforge").join("credentials.yaml"))
}

/// Loads [`CredentialsConfig`] from an explicit file path, or default when non-existent.
pub fn load_credentials_from_path(path: &Path) -> Result<CredentialsConfig, ForgeError> {
    if !path.is_file() {
        return Ok(CredentialsConfig::default());
    }
    let content = fs::read_to_string(path).map_err(|source| ForgeError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    CredentialsConfig::parse_yaml(&content)
}

/// Saves [`CredentialsConfig`] to an explicit file path.
pub fn save_credentials_to_path(
    credentials: &CredentialsConfig,
    path: &Path,
) -> Result<(), ForgeError> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|source| ForgeError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }
    let yaml_str = credentials.to_yaml();
    fs::write(path, yaml_str).map_err(|source| ForgeError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = std::fs::set_permissions(path, perms);
    }

    Ok(())
}

/// Loads credentials from the default location `~/.eforge/credentials.yaml`.
pub fn load_credentials() -> Result<CredentialsConfig, ForgeError> {
    let path = default_credentials_path().ok_or_else(|| ForgeError::Config {
        message: "could not determine user home directory for credentials".to_string(),
    })?;
    load_credentials_from_path(&path)
}

/// Saves credentials to the default location `~/.eforge/credentials.yaml`.
pub fn save_credentials(credentials: &CredentialsConfig) -> Result<(), ForgeError> {
    let path = default_credentials_path().ok_or_else(|| ForgeError::Config {
        message: "could not determine user home directory for credentials".to_string(),
    })?;
    save_credentials_to_path(credentials, &path)
}

/// Resolves the API token according to the precedence hierarchy:
/// 1. `EFORGE_API_TOKEN` environment variable (if non-empty)
/// 2. Stored credential for `server_url` in credentials config
/// 3. Stored credential for `default_server` in credentials config
/// 4. Stored credential if exactly one server is configured
pub fn resolve_token(server_url: Option<&str>) -> Option<String> {
    if let Ok(token) = std::env::var("EFORGE_API_TOKEN") {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let config = load_credentials().ok()?;
    config.resolve_token_for(server_url)
}

fn unquote(s: &str) -> &str {
    let trimmed = s.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

fn escape_yaml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_yaml_roundtrip() {
        let mut config = CredentialsConfig::new();
        config.default_server = Some("https://forge.elcode.id".to_string());
        config.set_credential(ServerCredential {
            server_url: "https://forge.elcode.id".to_string(),
            token: "efg_live_token123".to_string(),
            username: Some("luki".to_string()),
            expires_at: Some(1800000000),
        });
        config.set_credential(ServerCredential {
            server_url: "https://staging.forge.elcode.id".to_string(),
            token: "efg_staging_token456".to_string(),
            username: None,
            expires_at: None,
        });

        let yaml = config.to_yaml();
        let parsed = CredentialsConfig::parse_yaml(&yaml);
        assert!(parsed.is_ok());
        if let Ok(p) = parsed {
            assert_eq!(config, p);
        }
    }

    #[test]
    fn test_credentials_json_roundtrip() {
        let mut config = CredentialsConfig::new();
        config.set_credential(ServerCredential {
            server_url: "http://localhost:8080".to_string(),
            token: "local_dev_token".to_string(),
            username: Some("dev".to_string()),
            expires_at: None,
        });

        let json = config.to_json();
        assert!(json.is_ok());
        if let Ok(json_str) = json {
            let parsed = CredentialsConfig::parse_json(&json_str);
            assert!(parsed.is_ok());
            if let Ok(p) = parsed {
                assert_eq!(config, p);
            }

            // Also test parse_yaml handles JSON gracefully
            let parsed_from_yaml_fn = CredentialsConfig::parse_yaml(&json_str);
            assert!(parsed_from_yaml_fn.is_ok());
            if let Ok(p) = parsed_from_yaml_fn {
                assert_eq!(config, p);
            }
        }
    }

    #[test]
    fn test_token_resolution_precedence() {
        let mut config = CredentialsConfig::new();
        config.default_server = Some("https://prod.forge.id".to_string());
        config.set_credential(ServerCredential {
            server_url: "https://prod.forge.id".to_string(),
            token: "prod_token".to_string(),
            username: None,
            expires_at: None,
        });
        config.set_credential(ServerCredential {
            server_url: "https://dev.forge.id".to_string(),
            token: "dev_token".to_string(),
            username: None,
            expires_at: None,
        });

        // Exact match
        assert_eq!(
            config.resolve_token_for(Some("https://dev.forge.id")),
            Some("dev_token".to_string())
        );
        // Default server fallback
        assert_eq!(
            config.resolve_token_for(None),
            Some("prod_token".to_string())
        );
        // Unknown server returns None
        assert_eq!(
            config.resolve_token_for(Some("https://unknown.forge.id")),
            None
        );
    }

    #[test]
    fn test_save_and_load_file() {
        let dir = std::env::temp_dir().join(format!("eforge_test_cred_{}", std::process::id()));
        let file_path = dir.join("sub").join("credentials.yaml");

        let mut config = CredentialsConfig::new();
        config.set_credential(ServerCredential {
            server_url: "https://forge.elcode.id".to_string(),
            token: "test_token_abc".to_string(),
            username: Some("tester".to_string()),
            expires_at: Some(1900000000),
        });

        let save_res = save_credentials_to_path(&config, &file_path);
        assert!(save_res.is_ok());

        let loaded = load_credentials_from_path(&file_path);
        assert!(loaded.is_ok());
        if let Ok(l) = loaded {
            assert_eq!(config, l);
        }

        // Clean up
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let dir = std::env::temp_dir().join("nonexistent_eforge_creds_xyz.yaml");
        let loaded = load_credentials_from_path(&dir);
        assert!(loaded.is_ok());
        if let Ok(l) = loaded {
            assert_eq!(l, CredentialsConfig::default());
        }
    }
}
