//! `forge-ai` — AI provider abstraction, context retrieval builder, and cost controls for ELCODE Forge (`eforge`).
//!
//! This crate depends ONLY on `forge-core` and approved external crates (`serde`, `serde_json`, `ureq`, `blake3`).

pub mod factory;
pub mod mock;
pub mod provider;
pub mod providers;
pub mod types;

pub use factory::{build_provider, mask_secret};
pub use mock::MockProvider;
pub use provider::{ensure_ai_enabled, AiProvider};
pub use providers::{AnthropicProvider, GeminiProvider, OpenAiCompatProvider};
pub use types::{AiMessage, AiRequest, AiResponse, AiRole, TokenUsage};
