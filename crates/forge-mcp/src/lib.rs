//! `forge-mcp` — Native Model Context Protocol (MCP) server exposing `eforge_*` tools.

pub mod boundary;

pub use boundary::{
    detect_prompt_injection_signals, wrap_untrusted_payload, UntrustedEnvelope,
    UNTRUSTED_REPO_DATA_LEVEL, UNTRUSTED_SECURITY_NOTICE,
};
pub use forge_core::Repository;
