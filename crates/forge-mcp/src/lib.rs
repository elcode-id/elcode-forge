//! `forge-mcp` — Native Model Context Protocol (MCP) server exposing `eforge_*` tools.

pub mod boundary;
pub mod server;
pub mod tools;

pub use boundary::{
    detect_prompt_injection_signals, wrap_untrusted_payload, UntrustedEnvelope,
    UNTRUSTED_REPO_DATA_LEVEL, UNTRUSTED_SECURITY_NOTICE,
};
pub use forge_core::Repository;
pub use server::{handle_jsonrpc_line, serve_mcp_stdio};
pub use tools::{call_mcp_tool, list_mcp_tools, McpToolDescriptor};
