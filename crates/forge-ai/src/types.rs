use serde::{Deserialize, Serialize};

/// Role assigned to a message in an AI conversation turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiRole {
    System,
    User,
    Assistant,
}

/// A single message within an AI request or transcript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: AiRole,
    pub content: String,
}

impl AiMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::Assistant,
            content: content.into(),
        }
    }
}

/// Request envelope sent to an [`AiProvider`](crate::AiProvider).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiRequest {
    pub system: String,
    pub messages: Vec<AiMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl AiRequest {
    pub fn new(system: impl Into<String>, messages: Vec<AiMessage>) -> Self {
        Self {
            system: system.into(),
            messages,
            max_tokens: 1024,
            temperature: 0.0,
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }
}

/// Token accounting metrics returned by an [`AiProvider`](crate::AiProvider).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl TokenUsage {
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
        }
    }

    pub fn total(&self) -> u32 {
        self.input_tokens.saturating_add(self.output_tokens)
    }
}

/// Canonical response payload returned by an [`AiProvider`](crate::AiProvider).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiResponse {
    pub text: String,
    pub usage: TokenUsage,
    pub model: String,
}

impl AiResponse {
    pub fn new(text: impl Into<String>, usage: TokenUsage, model: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            usage,
            model: model.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_constructors_and_token_usage_total() {
        let msg = AiMessage::user("Analyze this module");
        assert_eq!(msg.role, AiRole::User);
        assert_eq!(msg.content, "Analyze this module");

        let usage = TokenUsage::new(150, 50);
        assert_eq!(usage.total(), 200);

        let req = AiRequest::new("System prompt", vec![msg])
            .with_max_tokens(2048)
            .with_temperature(0.2);
        assert_eq!(req.max_tokens, 2048);
        assert!((req.temperature - 0.2).abs() < 0.001);
    }
}
