use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ForgeError;
use crate::ir::Severity;

/// Primary configuration filename (PRD v1.1 standard).
pub const PRIMARY_CONFIG_FILE: &str = "eforge.config.yaml";

/// Legacy fallback configuration filename for backward compatibility.
pub const FALLBACK_CONFIG_FILE: &str = "forge.config.yaml";

/// Per-rule override settings declared under `rules:` in `eforge.config.yaml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleOverrideConfig {
    pub enabled: Option<bool>,
    pub severity: Option<Severity>,
    pub max_lines: Option<u32>,
}

/// Supported AI provider categories in ELCODE Forge (PRD §19).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiProviderKind {
    #[serde(
        rename = "openai-compatible",
        alias = "openai_compatible",
        alias = "openai"
    )]
    OpenAiCompatible,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "gemini")]
    Gemini,
}

/// Configuration settings for the optional AI layer (`ai:` in `eforge.config.yaml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    pub provider: Option<AiProviderKind>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,
    pub max_tokens: u32,
    pub max_context_tokens: u32,
    pub temperature: Option<f32>,
    pub timeout_secs: u64,
    pub cache_enabled: bool,
    pub budget_daily_usd: Option<f64>,
    pub price_input_per_1k: Option<f64>,
    pub price_output_per_1k: Option<f64>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: None,
            model: None,
            base_url: None,
            api_key_env: None,
            max_tokens: 1024,
            max_context_tokens: 8000,
            temperature: Some(0.0),
            timeout_secs: 60,
            cache_enabled: true,
            budget_daily_usd: None,
            price_input_per_1k: None,
            price_output_per_1k: None,
        }
    }
}

/// Configuration for native Git hooks automation (`eforge hook`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HooksConfig {
    #[serde(default)]
    pub pre_commit: Option<HookActionConfig>,
    #[serde(default)]
    pub pre_push: Option<HookActionConfig>,
}

/// Action settings for a specific Git hook lifecycle event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookActionConfig {
    #[serde(default = "default_hook_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub fail_on: Option<Severity>,
    #[serde(default)]
    pub fast: Option<bool>,
    #[serde(default)]
    pub staged_only: Option<bool>,
}

fn default_hook_enabled() -> bool {
    true
}

impl Default for HookActionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fail_on: Some(Severity::High),
            fast: Some(true),
            staged_only: Some(true),
        }
    }
}

/// Deterministic configuration model for ELCODE Forge (`eforge`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForgeConfig {
    pub version: u32,
    pub project_name: Option<String>,
    pub languages: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub git_enabled: bool,
    pub ai_enabled: bool,
    #[serde(default)]
    pub ai: AiConfig,
    pub fail_on_severity: Option<Severity>,
    #[serde(default)]
    pub rules: BTreeMap<String, RuleOverrideConfig>,
    #[serde(default)]
    pub rule_packs: Vec<String>,
    #[serde(default)]
    pub custom_rule_paths: Vec<String>,
    #[serde(default)]
    pub hooks: Option<HooksConfig>,
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
            ai: AiConfig::default(),
            fail_on_severity: None,
            rules: BTreeMap::new(),
            rule_packs: Vec::new(),
            custom_rule_paths: Vec::new(),
            hooks: None,
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
/// `git.enabled`, `ai.enabled`, `ignore`, `rules.<rule_id>.{enabled,severity,max_lines}`) and
/// flat keys (`project_name`, `languages`, `ignore_patterns`, `git_enabled`, `ai_enabled`,
/// `fail_on_severity`).
pub fn parse_config_yaml(content: &str) -> Result<ForgeConfig, ForgeError> {
    let mut config = ForgeConfig::default();
    let mut current_section: Option<&str> = None;
    let mut active_list: Option<&str> = None;
    let mut active_rule_id: Option<(String, usize)> = None;
    let mut active_ai_sub: Option<(String, usize)> = None;
    let mut active_hook_sub: Option<(String, usize)> = None;
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
                Some("rules_extends") => config.rule_packs.push(value.to_string()),
                Some("rules_custom_packs") => config.custom_rule_paths.push(value.to_string()),
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

        let key = unquote(raw_key.trim());
        let val = unquote(raw_val.trim());

        if indent == 0 {
            current_section = None;
            active_list = None;
            active_rule_id = None;
            active_ai_sub = None;
            active_hook_sub = None;

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
                    let b = parse_bool(val, line_no)?;
                    config.ai_enabled = b;
                    config.ai.enabled = b;
                }
                "fail_on_severity" => {
                    if !val.is_empty() {
                        config.fail_on_severity = Some(parse_severity(val, line_no)?);
                    }
                }
                "languages" => {
                    if let Some(items) = parse_inline_array(val) {
                        config.languages.extend(items);
                    } else {
                        active_list = Some("languages");
                    }
                }
                "ignore" | "ignore_patterns" => {
                    if let Some(items) = parse_inline_array(val) {
                        if !custom_ignores_started {
                            config.ignore_patterns.clear();
                            custom_ignores_started = true;
                        }
                        config.ignore_patterns.extend(items);
                    } else {
                        active_list = Some("ignore");
                    }
                }
                "rule_packs" => {
                    if let Some(items) = parse_inline_array(val) {
                        config.rule_packs.extend(items);
                    } else {
                        active_list = Some("rules_extends");
                    }
                }
                "custom_rules" | "custom_rule_paths" => {
                    if let Some(items) = parse_inline_array(val) {
                        config.custom_rule_paths.extend(items);
                    } else {
                        active_list = Some("rules_custom_packs");
                    }
                }
                "project" => current_section = Some("project"),
                "analysis" => current_section = Some("analysis"),
                "git" => current_section = Some("git"),
                "ai" => current_section = Some("ai"),
                "rules" => current_section = Some("rules"),
                "scoring" => current_section = Some("scoring"),
                "hooks" => {
                    current_section = Some("hooks");
                    if config.hooks.is_none() {
                        config.hooks = Some(HooksConfig::default());
                    }
                }
                _ => {
                    return Err(ForgeError::Config {
                        message: format!("line {line_no}: unknown top-level config key '{key}'"),
                    });
                }
            }
        } else {
            if let Some((ref sub_name, sub_indent)) = active_ai_sub {
                if indent > sub_indent {
                    check_no_raw_secret(key, line_no)?;
                    match sub_name.as_str() {
                        "budget" => match key {
                            "daily_usd" | "daily" => {
                                let parsed =
                                    val.parse::<f64>().map_err(|_| ForgeError::Config {
                                        message: format!(
                                        "line {line_no}: invalid float for budget daily_usd '{val}'"
                                    ),
                                    })?;
                                config.ai.budget_daily_usd = Some(parsed);
                            }
                            _ => {}
                        },
                        "pricing" => match key {
                            "input_per_1k" | "input" => {
                                let parsed = val.parse::<f64>().map_err(|_| ForgeError::Config {
                                    message: format!(
                                        "line {line_no}: invalid float for pricing input_per_1k '{val}'"
                                    ),
                                })?;
                                config.ai.price_input_per_1k = Some(parsed);
                            }
                            "output_per_1k" | "output" => {
                                let parsed = val.parse::<f64>().map_err(|_| ForgeError::Config {
                                    message: format!(
                                        "line {line_no}: invalid float for pricing output_per_1k '{val}'"
                                    ),
                                })?;
                                config.ai.price_output_per_1k = Some(parsed);
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                    continue;
                }
                active_ai_sub = None;
            }

            if let Some((ref hook_name, hook_indent)) = active_hook_sub {
                if indent > hook_indent {
                    let hooks = config.hooks.get_or_insert_with(HooksConfig::default);
                    let target_hook = match hook_name.as_str() {
                        "pre_commit" | "pre-commit" => hooks
                            .pre_commit
                            .get_or_insert_with(HookActionConfig::default),
                        "pre_push" | "pre-push" => {
                            hooks.pre_push.get_or_insert_with(HookActionConfig::default)
                        }
                        _ => {
                            return Err(ForgeError::Config {
                                message: format!("line {line_no}: unknown hook '{hook_name}'"),
                            });
                        }
                    };

                    match key {
                        "enabled" => target_hook.enabled = parse_bool(val, line_no)?,
                        "fail_on" | "fail_on_severity" => {
                            if !val.is_empty() {
                                target_hook.fail_on = Some(parse_severity(val, line_no)?);
                            }
                        }
                        "fast" => target_hook.fast = Some(parse_bool(val, line_no)?),
                        "staged_only" | "staged" => {
                            target_hook.staged_only = Some(parse_bool(val, line_no)?)
                        }
                        _ => {
                            return Err(ForgeError::Config {
                                message: format!(
                                    "line {line_no}: unsupported key '{key}' in hook '{hook_name}'"
                                ),
                            });
                        }
                    }
                    continue;
                }
                active_hook_sub = None;
            }

            if let Some((ref rule_id, rule_indent)) = active_rule_id {
                if indent > rule_indent {
                    let entry = config.rules.entry(rule_id.clone()).or_default();
                    match key {
                        "enabled" => entry.enabled = Some(parse_bool(val, line_no)?),
                        "severity" => entry.severity = Some(parse_severity(val, line_no)?),
                        "max_lines" | "threshold" => {
                            let parsed = val.parse::<u32>().map_err(|_| ForgeError::Config {
                                message: format!(
                                    "line {line_no}: invalid integer threshold '{val}'"
                                ),
                            })?;
                            entry.max_lines = Some(parsed);
                        }
                        _ => {}
                    }
                    continue;
                }
                active_rule_id = None;
            }

            match (current_section, key) {
                (Some("project"), "name") => {
                    if !val.is_empty() {
                        config.project_name = Some(val.to_string());
                    }
                }
                (Some("project"), "preset") => {}
                (Some("analysis"), "languages") => {
                    active_list = Some("languages");
                }
                (Some("git"), "enabled") => {
                    config.git_enabled = parse_bool(val, line_no)?;
                }
                (Some("ai"), ai_key) => {
                    check_no_raw_secret(ai_key, line_no)?;
                    match ai_key {
                        "enabled" => {
                            let b = parse_bool(val, line_no)?;
                            config.ai.enabled = b;
                            config.ai_enabled = b;
                        }
                        "provider" => {
                            config.ai.provider = Some(parse_provider_kind(val, line_no)?);
                        }
                        "model" => {
                            if !val.is_empty() {
                                config.ai.model = Some(val.to_string());
                            }
                        }
                        "base_url" => {
                            if !val.is_empty() {
                                config.ai.base_url = Some(validate_base_url(val, line_no)?);
                            }
                        }
                        "api_key_env" => {
                            if !val.is_empty() {
                                config.ai.api_key_env = Some(val.to_string());
                            }
                        }
                        "max_tokens" => {
                            let parsed = val.parse::<u32>().map_err(|_| ForgeError::Config {
                                message: format!(
                                    "line {line_no}: invalid integer for max_tokens '{val}'"
                                ),
                            })?;
                            config.ai.max_tokens = parsed;
                        }
                        "max_context_tokens" => {
                            let parsed = val.parse::<u32>().map_err(|_| ForgeError::Config {
                                message: format!(
                                    "line {line_no}: invalid integer for max_context_tokens '{val}'"
                                ),
                            })?;
                            config.ai.max_context_tokens = parsed;
                        }
                        "temperature" => {
                            let parsed = val.parse::<f32>().map_err(|_| ForgeError::Config {
                                message: format!(
                                    "line {line_no}: invalid float for temperature '{val}'"
                                ),
                            })?;
                            config.ai.temperature = Some(parsed);
                        }
                        "timeout_secs" | "timeout" => {
                            let parsed = val.parse::<u64>().map_err(|_| ForgeError::Config {
                                message: format!(
                                    "line {line_no}: invalid integer for timeout_secs '{val}'"
                                ),
                            })?;
                            config.ai.timeout_secs = parsed;
                        }
                        "cache" | "cache_enabled" => {
                            config.ai.cache_enabled = parse_bool(val, line_no)?;
                        }
                        "budget" => {
                            active_ai_sub = Some(("budget".to_string(), indent));
                        }
                        "budget.daily_usd" => {
                            let parsed = val.parse::<f64>().map_err(|_| ForgeError::Config {
                                message: format!(
                                    "line {line_no}: invalid float for budget daily_usd '{val}'"
                                ),
                            })?;
                            config.ai.budget_daily_usd = Some(parsed);
                        }
                        "pricing" => {
                            active_ai_sub = Some(("pricing".to_string(), indent));
                        }
                        "pricing.input_per_1k" => {
                            let parsed = val.parse::<f64>().map_err(|_| ForgeError::Config {
                                message: format!(
                                    "line {line_no}: invalid float for pricing input_per_1k '{val}'"
                                ),
                            })?;
                            config.ai.price_input_per_1k = Some(parsed);
                        }
                        "pricing.output_per_1k" => {
                            let parsed = val.parse::<f64>().map_err(|_| ForgeError::Config {
                                message: format!(
                                    "line {line_no}: invalid float for pricing output_per_1k '{val}'"
                                ),
                            })?;
                            config.ai.price_output_per_1k = Some(parsed);
                        }
                        _ => {
                            return Err(ForgeError::Config {
                                message: format!(
                                    "line {line_no}: unsupported key '{ai_key}' in 'ai' section"
                                ),
                            });
                        }
                    }
                }
                (Some("scoring"), _) => {}
                (Some("hooks"), hook_key) => match hook_key {
                    "pre_commit" | "pre-commit" | "pre_push" | "pre-push" => {
                        active_hook_sub = Some((hook_key.to_string(), indent));
                        let hooks = config.hooks.get_or_insert_with(HooksConfig::default);
                        if matches!(hook_key, "pre_commit" | "pre-commit")
                            && hooks.pre_commit.is_none()
                        {
                            hooks.pre_commit = Some(HookActionConfig::default());
                        } else if matches!(hook_key, "pre_push" | "pre-push")
                            && hooks.pre_push.is_none()
                        {
                            hooks.pre_push = Some(HookActionConfig::default());
                        }
                    }
                    _ => {
                        return Err(ForgeError::Config {
                            message: format!(
                                "line {line_no}: unsupported hook key '{hook_key}' in 'hooks' section"
                            ),
                        });
                    }
                },
                (Some("rules"), "extends") => {
                    if let Some(items) = parse_inline_array(val) {
                        config.rule_packs.extend(items);
                    } else {
                        active_list = Some("rules_extends");
                    }
                }
                (Some("rules"), "custom_packs" | "custom_rules" | "custom_rule_paths") => {
                    if let Some(items) = parse_inline_array(val) {
                        config.custom_rule_paths.extend(items);
                    } else {
                        active_list = Some("rules_custom_packs");
                    }
                }
                (Some("rules"), "fail_on_severity") => {
                    if !val.is_empty() {
                        config.fail_on_severity = Some(parse_severity(val, line_no)?);
                    }
                }
                (Some("rules"), rule_key) => {
                    active_list = None;
                    if val.is_empty() {
                        config.rules.entry(rule_key.to_string()).or_default();
                        active_rule_id = Some((rule_key.to_string(), indent));
                    } else if val == "off" || val == "false" {
                        config
                            .rules
                            .entry(rule_key.to_string())
                            .or_default()
                            .enabled = Some(false);
                    } else {
                        let sev = parse_severity(val, line_no)?;
                        let entry = config.rules.entry(rule_key.to_string()).or_default();
                        entry.enabled = Some(true);
                        entry.severity = Some(sev);
                    }
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

    if config.ai.enabled != config.ai_enabled {
        let unified = config.ai.enabled || config.ai_enabled;
        config.ai.enabled = unified;
        config.ai_enabled = unified;
    }

    Ok(config)
}

fn parse_inline_array(raw: &str) -> Option<Vec<String>> {
    let trimmed = raw.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let inner = &trimmed[1..trimmed.len() - 1];
        let items = inner
            .split(',')
            .map(|s| unquote(s.trim()).to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Some(items)
    } else {
        None
    }
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
        "medium" | "warning" | "warn" => Ok(Severity::Medium),
        "high" | "error" => Ok(Severity::High),
        "critical" => Ok(Severity::Critical),
        _ => Err(ForgeError::Config {
            message: format!("line {line_no}: invalid severity '{val}'"),
        }),
    }
}

fn check_no_raw_secret(key: &str, line_no: usize) -> Result<(), ForgeError> {
    let lower = key.to_ascii_lowercase();
    if lower == "api_key"
        || lower == "api-key"
        || lower == "token"
        || lower == "secret"
        || lower == "password"
    {
        return Err(ForgeError::Config {
            message: format!(
                "line {line_no}: do not store raw credentials in configuration; store credentials in an environment variable and reference it via 'api_key_env'"
            ),
        });
    }
    Ok(())
}

fn parse_provider_kind(val: &str, line_no: usize) -> Result<AiProviderKind, ForgeError> {
    match val.to_ascii_lowercase().as_str() {
        "openai-compatible" | "openai_compatible" | "openai" | "openrouter" | "ollama" => {
            Ok(AiProviderKind::OpenAiCompatible)
        }
        "anthropic" | "claude" => Ok(AiProviderKind::Anthropic),
        "gemini" | "google" => Ok(AiProviderKind::Gemini),
        _ => Err(ForgeError::Config {
            message: format!(
                "line {line_no}: unsupported ai provider '{val}'; expected 'openai-compatible', 'anthropic', or 'gemini'"
            ),
        }),
    }
}

fn validate_base_url(url: &str, line_no: usize) -> Result<String, ForgeError> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ForgeError::Config {
            message: format!(
                "line {line_no}: 'base_url' must start with 'http://' or 'https://', got '{url}'"
            ),
        });
    }
    Ok(url.to_string())
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
            assert!(!c.ai.enabled);
            assert_eq!(c.ai.max_tokens, 1024);
            assert_eq!(c.ai.max_context_tokens, 8000);
            assert!(c.ai.cache_enabled);
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
  arch/no-direct-db-in-ui:
    enabled: true
    severity: critical
  quality/max-file-lines:
    threshold: 250

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
            assert!(!cfg.ai.enabled);
            assert_eq!(cfg.fail_on_severity, Some(Severity::High));
            assert_eq!(
                cfg.rules
                    .get("arch/no-direct-db-in-ui")
                    .and_then(|r| r.severity),
                Some(Severity::Critical)
            );
            assert_eq!(
                cfg.rules
                    .get("quality/max-file-lines")
                    .and_then(|r| r.max_lines),
                Some(250)
            );
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
    fn parses_full_ai_configuration_section() {
        let yaml = r#"
version: 1
ai:
  enabled: true
  provider: openai-compatible
  model: gpt-4o
  base_url: https://api.openai.com/v1
  api_key_env: OPENAI_API_KEY
  max_tokens: 2048
  max_context_tokens: 12000
  temperature: 0.1
  timeout_secs: 45
  cache: true
  budget:
    daily_usd: 12.50
  pricing:
    input_per_1k: 0.0025
    output_per_1k: 0.010
"#;
        let res = parse_config_yaml(yaml);
        assert!(res.is_ok());
        if let Ok(cfg) = res {
            assert!(cfg.ai.enabled);
            assert!(cfg.ai_enabled);
            assert_eq!(cfg.ai.provider, Some(AiProviderKind::OpenAiCompatible));
            assert_eq!(cfg.ai.model.as_deref(), Some("gpt-4o"));
            assert_eq!(
                cfg.ai.base_url.as_deref(),
                Some("https://api.openai.com/v1")
            );
            assert_eq!(cfg.ai.api_key_env.as_deref(), Some("OPENAI_API_KEY"));
            assert_eq!(cfg.ai.max_tokens, 2048);
            assert_eq!(cfg.ai.max_context_tokens, 12000);
            assert_eq!(cfg.ai.temperature, Some(0.1));
            assert_eq!(cfg.ai.timeout_secs, 45);
            assert!(cfg.ai.cache_enabled);
            assert_eq!(cfg.ai.budget_daily_usd, Some(12.50));
            assert_eq!(cfg.ai.price_input_per_1k, Some(0.0025));
            assert_eq!(cfg.ai.price_output_per_1k, Some(0.010));
        }
    }

    #[test]
    fn rejects_raw_credentials_in_ai_section() {
        let yaml = r#"
version: 1
ai:
  enabled: true
  api_key: sk-proj-123456789
"#;
        let res = parse_config_yaml(yaml);
        assert!(res.is_err());
        if let Err(e) = res {
            let err_msg = e.to_string();
            assert!(err_msg.contains("do not store raw credentials in configuration"));
        }
    }

    #[test]
    fn rejects_invalid_base_url_scheme() {
        let yaml = r#"
version: 1
ai:
  enabled: true
  base_url: ftp://example.com/v1
"#;
        let res = parse_config_yaml(yaml);
        assert!(res.is_err());
        if let Err(e) = res {
            let err_msg = e.to_string();
            assert!(err_msg.contains("must start with 'http://' or 'https://'"));
        }
    }

    #[test]
    fn synchronizes_legacy_ai_enabled_flag() {
        let yaml = "version: 1\nai_enabled: true\n";
        let res = parse_config_yaml(yaml);
        assert!(res.is_ok());
        if let Ok(cfg) = res {
            assert!(cfg.ai_enabled);
            assert!(cfg.ai.enabled);
        }
    }

    #[test]
    fn parses_rules_extends_and_custom_packs() {
        let yaml = r#"
version: 1
rules:
  extends:
    - recommended
    - strict
  custom_packs:
    - .eforge/rules
    - rules/team-rules.yaml
  quality/max-file-lines:
    max_lines: 450
"#;
        let res = parse_config_yaml(yaml);
        assert!(res.is_ok());
        if let Ok(cfg) = res {
            assert_eq!(cfg.rule_packs, vec!["recommended", "strict"]);
            assert_eq!(
                cfg.custom_rule_paths,
                vec![".eforge/rules", "rules/team-rules.yaml"]
            );
            assert!(cfg.rules.contains_key("quality/max-file-lines"));
            if let Some(rule_override) = cfg.rules.get("quality/max-file-lines") {
                assert_eq!(rule_override.max_lines, Some(450));
            }
        }
    }

    #[test]
    fn parses_inline_arrays_for_extends_and_rules() {
        let yaml = r#"
version: 1
languages: ["typescript", "rust"]
rules:
  extends: ["recommended", "clean-architecture"]
  custom_packs: ["rules/team.yaml", "rules/strict.json"]
"#;
        let res = parse_config_yaml(yaml);
        assert!(res.is_ok());
        if let Ok(cfg) = res {
            assert_eq!(cfg.languages, vec!["typescript", "rust"]);
            assert_eq!(cfg.rule_packs, vec!["recommended", "clean-architecture"]);
            assert_eq!(
                cfg.custom_rule_paths,
                vec!["rules/team.yaml", "rules/strict.json"]
            );
        }
    }

    #[test]
    fn parses_hooks_configuration_cleanly() {
        let yaml = r#"
version: 1
hooks:
  pre_commit:
    enabled: true
    fail_on: high
    fast: true
    staged_only: true
  pre_push:
    enabled: false
    fail_on: critical
"#;
        let res = parse_config_yaml(yaml);
        assert!(res.is_ok());
        if let Ok(cfg) = res {
            assert!(cfg.hooks.is_some());
            if let Some(hooks) = cfg.hooks {
                assert!(hooks.pre_commit.is_some());
                if let Some(pre_commit) = hooks.pre_commit {
                    assert!(pre_commit.enabled);
                    assert_eq!(pre_commit.fail_on, Some(Severity::High));
                    assert_eq!(pre_commit.fast, Some(true));
                    assert_eq!(pre_commit.staged_only, Some(true));
                }

                assert!(hooks.pre_push.is_some());
                if let Some(pre_push) = hooks.pre_push {
                    assert!(!pre_push.enabled);
                    assert_eq!(pre_push.fail_on, Some(Severity::Critical));
                }
            }
        }
    }

    #[test]
    fn rejects_invalid_config_syntax() {
        let bad_yaml = "version: not_a_number\n";
        let res = parse_config_yaml(bad_yaml);
        assert!(matches!(res, Err(ForgeError::Config { .. })));
    }
}
