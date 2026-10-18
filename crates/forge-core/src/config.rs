use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ForgeError;
use crate::ir::Severity;

/// Primary configuration filename (PRD v1.1 standard).
pub const PRIMARY_CONFIG_FILE: &str = "eforge.config.yaml";

/// Legacy fallback configuration filename for backward compatibility.
pub const FALLBACK_CONFIG_FILE: &str = "forge.config.yaml";

/// Deterministic configuration model for ELCODE Forge (`eforge`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgeConfig {
    pub version: u32,
    pub project_name: Option<String>,
    pub languages: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub git_enabled: bool,
    pub ai_enabled: bool,
    pub fail_on_severity: Option<Severity>,
}

impl Default for ForgeConfig {
    fn default() -> Self {
        Self {
            version: 1,
            project_name: None,
            languages: Vec::new(),
            ignore_patterns: vec![
                "node_modules".to_string(),
                "dist".to_string(),
                ".next".to_string(),
                ".eforge".to_string(),
            ],
            git_enabled: true,
            ai_enabled: false,
            fail_on_severity: None,
        }
    }
}

/// Loads `eforge.config.yaml` from `repo_root`, falling back to `forge.config.yaml` if not found,
/// or returning [`ForgeConfig::default()`] when neither file exists.
pub fn load_config(repo_root: &Path) -> Result<ForgeConfig, ForgeError> {
    let primary_path = repo_root.join(PRIMARY_CONFIG_FILE);
    if primary_path.is_file() {
        let content = fs::read_to_string(&primary_path).map_err(|source| ForgeError::Io {
            path: primary_path,
            source,
        })?;
        return parse_config_yaml(&content);
    }

    let fallback_path = repo_root.join(FALLBACK_CONFIG_FILE);
    if fallback_path.is_file() {
        let content = fs::read_to_string(&fallback_path).map_err(|source| ForgeError::Io {
            path: fallback_path,
            source,
        })?;
        return parse_config_yaml(&content);
    }

    Ok(ForgeConfig::default())
}

/// Parses an `eforge.config.yaml` document into [`ForgeConfig`].
///
/// Supports both the nested PRD Section 28 YAML schema (`project.name`, `analysis.languages`,
/// `git.enabled`, `ai.enabled`, `ignore`) and flat keys (`project_name`, `languages`,
/// `ignore_patterns`, `git_enabled`, `ai_enabled`, `fail_on_severity`).
pub fn parse_config_yaml(content: &str) -> Result<ForgeConfig, ForgeError> {
    let mut config = ForgeConfig::default();
    let mut current_section: Option<&str> = None;
    let mut active_list: Option<&str> = None;
    let mut custom_ignores_started = false;

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let without_comment = match raw_line.find('#') {
            Some(pos) => &raw_line[..pos],
            None => raw_line,
        };
        let trimmed = without_comment.trim();
        if trimmed.is_empty() || trimmed == "---" {
            continue;
        }

        let indent = without_comment
            .chars()
            .take_while(|c| c.is_whitespace())
            .count();

        if let Some(item) = trimmed.strip_prefix('-') {
            let value = unquote(item.trim());
            if value.is_empty() {
                return Err(ForgeError::Config {
                    message: format!("line {line_no}: empty list item in configuration"),
                });
            }
            match active_list {
                Some("languages") => config.languages.push(value.to_string()),
                Some("ignore") => {
                    if !custom_ignores_started {
                        config.ignore_patterns.clear();
                        custom_ignores_started = true;
                    }
                    config.ignore_patterns.push(value.to_string());
                }
                Some(_) => {}
                None => {
                    return Err(ForgeError::Config {
                        message: format!("line {line_no}: unexpected list item '{value}'"),
                    });
                }
            }
            continue;
        }

        let Some((raw_key, raw_val)) = trimmed.split_once(':') else {
            return Err(ForgeError::Config {
                message: format!("line {line_no}: expected 'key: value' syntax"),
            });
        };

        let key = raw_key.trim();
        let val = unquote(raw_val.trim());

        if indent == 0 {
            current_section = None;
            active_list = None;

            match key {
                "version" => {
                    if val.is_empty() {
                        return Err(ForgeError::Config {
                            message: format!("line {line_no}: missing value for 'version'"),
                        });
                    }
                    let parsed = val.parse::<u32>().map_err(|_| ForgeError::Config {
                        message: format!("line {line_no}: invalid integer version '{val}'"),
                    })?;
                    if parsed == 0 {
                        return Err(ForgeError::Config {
                            message: format!("line {line_no}: config version must be >= 1"),
                        });
                    }
                    config.version = parsed;
                }
                "project_name" => {
                    if !val.is_empty() {
                        config.project_name = Some(val.to_string());
                    }
                }
                "git_enabled" => {
                    config.git_enabled = parse_bool(val, line_no)?;
                }
                "ai_enabled" => {
                    config.ai_enabled = parse_bool(val, line_no)?;
                }
                "fail_on_severity" => {
                    if !val.is_empty() {
                        config.fail_on_severity = Some(parse_severity(val, line_no)?);
                    }
                }
                "languages" => {
                    active_list = Some("languages");
                }
                "ignore" | "ignore_patterns" => {
                    active_list = Some("ignore");
                }
                "project" => current_section = Some("project"),
                "analysis" => current_section = Some("analysis"),
                "git" => current_section = Some("git"),
                "ai" => current_section = Some("ai"),
                "rules" => current_section = Some("rules"),
                _ => {
                    return Err(ForgeError::Config {
                        message: format!("line {line_no}: unknown top-level config key '{key}'"),
                    });
                }
            }
        } else {
            match (current_section, key) {
                (Some("project"), "name") => {
                    if !val.is_empty() {
                        config.project_name = Some(val.to_string());
                    }
                }
                (Some("analysis"), "languages") => {
                    active_list = Some("languages");
                }
                (Some("git"), "enabled") => {
                    config.git_enabled = parse_bool(val, line_no)?;
                }
                (Some("ai"), "enabled") => {
                    config.ai_enabled = parse_bool(val, line_no)?;
                }
                (Some("rules"), "extends") => {
                    active_list = Some("rules_extends");
                }
                (Some("rules"), "fail_on_severity") => {
                    if !val.is_empty() {
                        config.fail_on_severity = Some(parse_severity(val, line_no)?);
                    }
                }
                (Some("rules"), _) => {
                    active_list = None;
                }
                _ => {
                    return Err(ForgeError::Config {
                        message: format!(
                            "line {line_no}: unsupported nested key '{key}' in section {:?}",
                            current_section
                        ),
                    });
                }
            }
        }
    }

    Ok(config)
}

fn unquote(input: &str) -> &str {
    if input.len() >= 2
        && ((input.starts_with('"') && input.ends_with('"'))
            || (input.starts_with('\'') && input.ends_with('\'')))
    {
        &input[1..input.len() - 1]
    } else {
        input
    }
}

fn parse_bool(val: &str, line_no: usize) -> Result<bool, ForgeError> {
    match val {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(ForgeError::Config {
            message: format!("line {line_no}: expected boolean 'true' or 'false', got '{val}'"),
        }),
    }
}

fn parse_severity(val: &str, line_no: usize) -> Result<Severity, ForgeError> {
    match val.to_ascii_lowercase().as_str() {
        "info" => Ok(Severity::Info),
        "low" => Ok(Severity::Low),
        "medium" => Ok(Severity::Medium),
        "high" => Ok(Severity::High),
        "critical" => Ok(Severity::Critical),
        _ => Err(ForgeError::Config {
            message: format!("line {line_no}: invalid severity '{val}'"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = env::temp_dir().join(format!("eforge_cfg_test_{label}_{nanos}"));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn returns_default_config_when_no_file_exists() {
        let dir = unique_temp_dir("default");
        let cfg = load_config(&dir);
        assert!(cfg.is_ok());
        if let Ok(c) = cfg {
            assert_eq!(c, ForgeConfig::default());
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn loads_primary_eforge_config_yaml_and_ignores_fallback() {
        let dir = unique_temp_dir("primary");
        let primary_yaml = r#"
version: 1

project:
  name: eforge-primary-app

analysis:
  languages:
    - typescript
    - javascript

rules:
  extends:
    - recommended
  fail_on_severity: high

ai:
  enabled: false

git:
  enabled: true

ignore:
  - node_modules
  - dist
  - .next
"#;
        let fallback_yaml = "version: 1\nproject_name: legacy-app\n";
        assert!(fs::write(dir.join(PRIMARY_CONFIG_FILE), primary_yaml).is_ok());
        assert!(fs::write(dir.join(FALLBACK_CONFIG_FILE), fallback_yaml).is_ok());

        let loaded = load_config(&dir);
        assert!(loaded.is_ok());
        if let Ok(cfg) = loaded {
            assert_eq!(cfg.version, 1);
            assert_eq!(cfg.project_name.as_deref(), Some("eforge-primary-app"));
            assert_eq!(cfg.languages, vec!["typescript", "javascript"]);
            assert_eq!(cfg.ignore_patterns, vec!["node_modules", "dist", ".next"]);
            assert!(cfg.git_enabled);
            assert!(!cfg.ai_enabled);
            assert_eq!(cfg.fail_on_severity, Some(Severity::High));
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn falls_back_to_forge_config_yaml_when_primary_is_absent() {
        let dir = unique_temp_dir("fallback");
        let fallback_yaml = r#"
version: 1
project:
  name: fallback-app
git:
  enabled: false
"#;
        assert!(fs::write(dir.join(FALLBACK_CONFIG_FILE), fallback_yaml).is_ok());

        let loaded = load_config(&dir);
        assert!(loaded.is_ok());
        if let Ok(cfg) = loaded {
            assert_eq!(cfg.project_name.as_deref(), Some("fallback-app"));
            assert!(!cfg.git_enabled);
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_invalid_config_syntax() {
        let bad_yaml = "version: not_a_number\n";
        let res = parse_config_yaml(bad_yaml);
        assert!(matches!(res, Err(ForgeError::Config { .. })));
    }
}
