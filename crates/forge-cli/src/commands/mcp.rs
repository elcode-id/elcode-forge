//! `eforge mcp` command — starts the native Model Context Protocol (MCP) JSON-RPC 2.0 stdio server.

use std::path::Path;

use forge_core::ForgeError;
use forge_mcp::serve_mcp_stdio;

/// Starts the `eforge mcp` stdio server rooted at `repo_root`.
pub fn run_mcp(repo_root: &Path) -> Result<(), ForgeError> {
    serve_mcp_stdio(repo_root)
}
