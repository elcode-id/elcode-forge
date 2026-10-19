use forge_core::config::{AiConfig, AiProviderKind};
use forge_core::error::ForgeError;

use crate::provider::{ensure_ai_enabled, AiProvider};
use crate::providers::anthropic::{
    AnthropicProvider, DEFAULT_ANTHROPIC_BASE_URL, DEFAULT_ANTHROPIC_MODEL,
};
use crate::providers::gemini::{GeminiProvider, DEFAULT_GEMINI_BASE_URL, DEFAULT_GEMINI_MODEL};
use crate::providers::openai_compat::OpenAiCompatProvider;

pub const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";

/// Masks sensitive API keys for display in status, error logs, and JSON outputs (PRD §36).
///
/// Leaves small edge identifiers while redacting the majority of characters.
pub fn mask_secret(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }
    let len = trimmed.len();
    if len <= 6 {
        return "******".to_string();
    }
    if len <= 10 {
        let prefix = &trimmed[..2];
        let suffix = &trimmed[len - 2..];
        return format!("{prefix}...{suffix}");
    }

    let prefix = &trimmed[..4];
    let suffix = &trimmed[len - 4..];
    format!("{prefix}...{suffix}")
}

/// Constructs a boxed [`AiProvider`] according to configuration and environment credentials.
///
/// Resolves API keys exclusively through `env_lookup` using `cfg.api_key_env` (or default vendor env vars).
pub fn build_provider(
    cfg: &AiConfig,
    env_lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Box<dyn AiProvider>, ForgeError> {
    ensure_ai_enabled(cfg)?;

    let kind = cfg.provider.unwrap_or(AiProviderKind::OpenAiCompatible);

    let default_env_var = match kind {
        AiProviderKind::OpenAiCompatible => "OPENAI_API_KEY",
        AiProviderKind::Anthropic => "ANTHROPIC_API_KEY",
        AiProviderKind::Gemini => "GEMINI_API_KEY",
    };

    let target_var_name = cfg.api_key_env.as_deref().unwrap_or(default_env_var);

    let api_key = env_lookup(target_var_name);

    match kind {
        AiProviderKind::OpenAiCompatible => {
            let base_url = cfg
                .base_url
                .clone()
                .unwrap_or_else(|| DEFAULT_OPENAI_BASE_URL.to_string());
            let model = cfg
                .model
                .clone()
                .unwrap_or_else(|| DEFAULT_OPENAI_MODEL.to_string());
            let provider = OpenAiCompatProvider::new(base_url, api_key, model, cfg.timeout_secs)?;
            Ok(Box::new(provider))
        }
        AiProviderKind::Anthropic => {
            let base_url = cfg
                .base_url
                .clone()
                .unwrap_or_else(|| DEFAULT_ANTHROPIC_BASE_URL.to_string());
            let model = cfg
                .model
                .clone()
                .unwrap_or_else(|| DEFAULT_ANTHROPIC_MODEL.to_string());
            let key = api_key.ok_or_else(|| ForgeError::Ai {
                message: format!(
                    "Anthropic provider requires an API key in environment variable '{target_var_name}'"
                ),
            })?;
            let provider = AnthropicProvider::new(base_url, key, model, cfg.timeout_secs)?;
            Ok(Box::new(provider))
        }
        AiProviderKind::Gemini => {
            let base_url = cfg
                .base_url
                .clone()
                .unwrap_or_else(|| DEFAULT_GEMINI_BASE_URL.to_string());
            let model = cfg
                .model
                .clone()
                .unwrap_or_else(|| DEFAULT_GEMINI_MODEL.to_string());
            let provider = GeminiProvider::new(base_url, api_key, model, cfg.timeout_secs)?;
            Ok(Box::new(provider))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_secret_redacts_credentials_safely() {
        assert_eq!(mask_secret(""), "<empty>");
        assert_eq!(mask_secret("short"), "******");
        assert_eq!(mask_secret("12345678"), "12...78");
        assert_eq!(
            mask_secret(concat!("sk-ant-", "api03-abcdefghijklmnop1234")),
            "sk-a...1234"
        );
    }

    #[test]
    fn build_provider_guards_disabled_ai() {
        let cfg = AiConfig {
            enabled: false,
            ..Default::default()
        };
        let res = build_provider(&cfg, &|_| None);
        assert!(res.is_err());
    }

    #[test]
    fn build_provider_creates_openai_compat_without_key_for_local_inference() {
        let cfg = AiConfig {
            enabled: true,
            provider: Some(AiProviderKind::OpenAiCompatible),
            base_url: Some("http://localhost:11434/v1".to_string()),
            model: Some("qwen2.5-coder:7b".to_string()),
            ..Default::default()
        };
        let res = build_provider(&cfg, &|_| None);
        assert!(res.is_ok());
        if let Ok(provider) = res {
            assert_eq!(provider.name(), "openai-compatible");
        }
    }

    #[test]
    fn build_provider_fails_for_anthropic_when_key_is_missing() {
        let cfg = AiConfig {
            enabled: true,
            provider: Some(AiProviderKind::Anthropic),
            api_key_env: Some("CUSTOM_ANTHROPIC_KEY".to_string()),
            ..Default::default()
        };
        let res = build_provider(&cfg, &|_| None);
        assert!(res.is_err());
        if let Err(e) = res {
            assert!(e.to_string().contains("CUSTOM_ANTHROPIC_KEY"));
        }
    }

    #[test]
    fn build_provider_succeeds_when_key_is_present() {
        let cfg = AiConfig {
            enabled: true,
            provider: Some(AiProviderKind::Anthropic),
            ..Default::default()
        };
        let env_fn = |var: &str| {
            if var == "ANTHROPIC_API_KEY" {
                Some("sk-ant-live-token".to_string())
            } else {
                None
            }
        };
        let res = build_provider(&cfg, &env_fn);
        assert!(res.is_ok());
        if let Ok(provider) = res {
            assert_eq!(provider.name(), "anthropic");
        }
    }
}
