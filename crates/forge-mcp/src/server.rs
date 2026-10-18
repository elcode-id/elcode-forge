use std::io::{self, BufRead, Write};
use std::path::Path;

use forge_core::ForgeError;
use serde_json::{json, Value};

use crate::tools::{call_mcp_tool, list_mcp_tools};

/// Processes a single JSON-RPC 2.0 line for the MCP server.
/// Returns `Some(json_response)` for requests and parse errors, or `None` for notifications/blank lines.
#[must_use]
pub fn handle_jsonrpc_line(repo_root: &Path, raw_line: &str) -> Option<String> {
    let trimmed = raw_line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let parsed: Value = match serde_json::from_str(trimmed) {
        Ok(val) => val,
        Err(err) => {
            let err_resp = json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": {
                    "code": -32700,
                    "message": format!("Parse error: {err}"),
                }
            });
            return serde_json::to_string(&err_resp).ok();
        }
    };

    let method = parsed.get("method").and_then(Value::as_str).unwrap_or("");
    let id = parsed.get("id").cloned();

    // JSON-RPC notifications (no `id` field) do not receive a response.
    if id.is_none() || method.starts_with("notifications/") {
        return None;
    }

    let req_id = id.unwrap_or(Value::Null);
    let params = parsed.get("params").cloned().unwrap_or_else(|| json!({}));

    let response = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "eforge-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }),
        "ping" => json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {}
        }),
        "tools/list" => {
            let tools: Vec<Value> = list_mcp_tools()
                .into_iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": t.input_schema,
                    })
                })
                .collect();
            json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "tools": tools
                }
            })
        }
        "tools/call" => {
            let tool_name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            match call_mcp_tool(repo_root, tool_name, &arguments) {
                Ok(envelope_val) => {
                    let pretty = serde_json::to_string_pretty(&envelope_val)
                        .unwrap_or_else(|_| envelope_val.to_string());
                    json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": pretty
                                }
                            ],
                            "isError": false
                        }
                    })
                }
                Err(err) => json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": format!("MCP tool execution error: {err}")
                            }
                        ],
                        "isError": true
                    }
                }),
            }
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {method}")
            }
        }),
    };

    serde_json::to_string(&response).ok()
}

/// Runs the blocking JSON-RPC 2.0 stdio transport loop for `eforge mcp`.
pub fn serve_mcp_stdio(repo_root: &Path) -> Result<(), ForgeError> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line_res in stdin.lock().lines() {
        let line = line_res.map_err(|source| ForgeError::Io {
            path: repo_root.to_path_buf(),
            source,
        })?;
        if let Some(resp) = handle_jsonrpc_line(repo_root, &line) {
            writeln!(stdout, "{resp}").map_err(|source| ForgeError::Io {
                path: repo_root.to_path_buf(),
                source,
            })?;
            stdout.flush().map_err(|source| ForgeError::Io {
                path: repo_root.to_path_buf(),
                source,
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_next_app() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("next-app")
    }

    #[test]
    fn initialize_and_tools_list_return_valid_jsonrpc_responses() {
        let root = fixture_next_app();
        let init_resp = handle_jsonrpc_line(
            &root,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        );
        assert!(init_resp.is_some());
        if let Some(raw) = init_resp {
            let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
            assert_eq!(v["result"]["serverInfo"]["name"], "eforge-mcp");
        }

        // Notification returns None
        let notif = handle_jsonrpc_line(
            &root,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        );
        assert!(notif.is_none());

        let list_resp =
            handle_jsonrpc_line(&root, r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
        assert!(list_resp.is_some());
        if let Some(raw) = list_resp {
            let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
            assert_eq!(v["result"]["tools"].as_array().map_or(0, Vec::len), 6);
        }
    }

    #[test]
    fn tools_call_returns_untrusted_envelope_content() {
        let root = fixture_next_app();
        let call_resp = handle_jsonrpc_line(
            &root,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"eforge_project_info","arguments":{}}}"#,
        );
        assert!(call_resp.is_some());
        if let Some(raw) = call_resp {
            let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
            assert_eq!(v["result"]["isError"], false);
            let text = v["result"]["content"][0]["text"].as_str().unwrap_or("");
            assert!(text.contains("untrusted_repository_data"));
        }
    }
}
