//! `ai` — AI command orchestration, context assembly, grounding checks, and cost controls for `forge-cli`.

pub mod orchestrate;

pub use orchestrate::{
    execute_explain_ai, AiExplainAnswer, AiExplainOptions, AiExplainOutput, GroundedClaim,
};
