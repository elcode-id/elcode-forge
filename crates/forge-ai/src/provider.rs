use forge_core::config::AiConfig;
use forge_core::error::ForgeError;

use crate::types::{AiRequest, AiResponse};

/// Trait abstracting LLM providers (PRD §19).
///
/// Implementations must adhere strictly to ELCODE Forge reliability standards:
/// zero panic, bounded memory allocation, and transparent error propagation via [`ForgeError::Ai`].
pub trait AiProvider: Send + Sync {
    /// Human-readable identifier of the provider implementation (e.g. `"openai-compatible"`, `"anthropic"`, `"gemini"`).
    fn name(&self) -> &'static str;

    /// Generates a single complete response synchronously.
    fn generate(&self, req: &AiRequest) -> Result<AiResponse, ForgeError>;

    /// Streams token chunks to `on_chunk` as they arrive, returning the complete aggregated [`AiResponse`].
    fn stream(
        &self,
        req: &AiRequest,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<AiResponse, ForgeError>;

    /// Computes dense vector embeddings for the provided input strings.
    fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ForgeError>;

    /// Generates structured output conforming strictly to the supplied JSON schema.
    fn structured_output(
        &self,
        req: &AiRequest,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, ForgeError>;
}

/// Verifies that AI capabilities are explicitly enabled in the configuration.
///
/// Returns an actionable diagnostic error [`ForgeError::Ai`] if `cfg.enabled` is `false`.
pub fn ensure_ai_enabled(cfg: &AiConfig) -> Result<(), ForgeError> {
    if !cfg.enabled {
        return Err(ForgeError::Ai {
            message: "AI capabilities are disabled. Enable them in 'eforge.config.yaml' under 'ai.enabled: true' or remove the AI flag.".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_ai_enabled_guards_disabled_state() {
        let mut cfg = AiConfig::default();
        cfg.enabled = false;
        let res = ensure_ai_enabled(&cfg);
        assert!(res.is_err());
        if let Err(e) = res {
            assert!(e.to_string().contains("AI capabilities are disabled"));
        }

        cfg.enabled = true;
        assert!(ensure_ai_enabled(&cfg).is_ok());
    }
}
