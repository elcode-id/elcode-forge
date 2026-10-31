//! `forge-ai` — AI provider abstraction, context retrieval builder, and cost controls for ELCODE Forge (`eforge`).
//!
//! This crate depends ONLY on `forge-core` and approved external crates (`serde`, `serde_json`, `ureq`, `blake3`).

pub mod budget;
pub mod cache;
pub mod context;
pub mod factory;
pub mod mock;
pub mod prompt;
pub mod provider;
pub mod providers;
pub mod tokens;
pub mod types;
pub mod usage;

pub use budget::{compute_cost_usd, enforce_daily_budget, preflight_check};
pub use cache::ResponseCache;
pub use context::{
    build_context, build_pruned_centrality_context, sections_from_repository, CodeSnippet,
    ContextInput, ContextPack, ContextSection, PrunedCentralityContext, PrunedModuleNode,
    SectionKind,
};
pub use factory::{build_provider, mask_secret};
pub use mock::MockProvider;
pub use prompt::{system_prompt_for_task, AiTask};
pub use provider::{ensure_ai_enabled, AiProvider};
pub use providers::{AnthropicProvider, GeminiProvider, OpenAiCompatProvider};
pub use tokens::estimate_tokens;
pub use types::{AiMessage, AiRequest, AiResponse, AiRole, TokenUsage};
pub use usage::{append_usage, summarize_usage, UsageRecord, UsageSummary};
