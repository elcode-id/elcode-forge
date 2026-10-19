use std::io::{BufRead, BufReader};

use forge_core::error::ForgeError;
use serde_json::json;

use crate::provider::AiProvider;
use crate::providers::http::{create_agent, map_ureq_error, read_bounded_body};
use crate::types::{AiRequest, AiResponse, AiRole, TokenUsage};

pub const DEFAULT_ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";
pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-3-5-sonnet-20241022";
pub const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Provider adapter for Anthropic Claude Messages API (PRD §19).
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    base_url: String,
    api_key: String,
    model: String,
    agent: ureq::Agent,
}

impl AnthropicProvider {
    pub fn new(
        base_url: String,
        api_key: String,
        model: String,
        timeout_secs: u64,
    ) -> Result<Self, ForgeError> {
        let trimmed_url = base_url.trim_end_matches('/').to_string();
        if !trimmed_url.starts_with("http://") && !trimmed_url.starts_with("https://") {
            return Err(ForgeError::Ai {
                message: format!(
                    "invalid base_url '{trimmed_url}': must start with 'http://' or 'https://'"
                ),
            });
        }
        if api_key.trim().is_empty() {
            return Err(ForgeError::Ai {
                message: "Anthropic provider requires a non-empty API key".to_string(),
            });
        }

        Ok(Self {
            base_url: trimmed_url,
            api_key,
            model,
            agent: create_agent(timeout_secs),
        })
    }

    fn prepare_messages(&self, req: &AiRequest) -> Vec<serde_json::Value> {
        let mut messages = Vec::new();
        for msg in &req.messages {
            let role_str = match msg.role {
                AiRole::System | AiRole::User => "user",
                AiRole::Assistant => "assistant",
            };
            messages.push(json!({
                "role": role_str,
                "content": msg.content,
            }));
        }
        if messages.is_empty() {
            messages.push(json!({
                "role": "user",
                "content": "Proceed with analysis.",
            }));
        }
        messages
    }
}

impl AiProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn generate(&self, req: &AiRequest) -> Result<AiResponse, ForgeError> {
        let endpoint = format!("{}/v1/messages", self.base_url);
        let mut payload = json!({
            "model": self.model,
            "messages": self.prepare_messages(req),
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
        });

        if !req.system.is_empty() {
            payload["system"] = json!(req.system);
        }

        let response = self
            .agent
            .post(&endpoint)
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", ANTHROPIC_VERSION)
            .set("Content-Type", "application/json")
            .send_json(payload)
            .map_err(|e| map_ureq_error("anthropic", e))?;

        let mut reader = response.into_reader();
        let body_str = read_bounded_body(&mut reader)?;

        let parsed: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| ForgeError::Ai {
                message: format!("failed to parse Anthropic JSON response: {e}"),
            })?;

        let mut text = String::new();
        if let Some(content_arr) = parsed["content"].as_array() {
            for block in content_arr {
                if block["type"] == "text" {
                    if let Some(t) = block["text"].as_str() {
                        text.push_str(t);
                    }
                }
            }
        }

        let input_tokens = parsed["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = parsed["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;
        let model = parsed["model"].as_str().unwrap_or(&self.model).to_string();

        Ok(AiResponse::new(
            text,
            TokenUsage::new(input_tokens, output_tokens),
            model,
        ))
    }

    fn stream(
        &self,
        req: &AiRequest,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<AiResponse, ForgeError> {
        let endpoint = format!("{}/v1/messages", self.base_url);
        let mut payload = json!({
            "model": self.model,
            "messages": self.prepare_messages(req),
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
            "stream": true,
        });

        if !req.system.is_empty() {
            payload["system"] = json!(req.system);
        }

        let response = self
            .agent
            .post(&endpoint)
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", ANTHROPIC_VERSION)
            .set("Content-Type", "application/json")
            .send_json(payload)
            .map_err(|e| map_ureq_error("anthropic", e))?;

        let reader = BufReader::new(response.into_reader());
        let mut full_text = String::new();
        let mut input_tokens = 0u32;
        let mut output_tokens = 0u32;

        for line_res in reader.lines() {
            let line = line_res.map_err(|e| ForgeError::Ai {
                message: format!("error reading Anthropic SSE stream line: {e}"),
            })?;

            let trimmed = line.trim();
            if let Some(data) = trimmed.strip_prefix("data:") {
                let content = data.trim();
                if let Ok(evt) = serde_json::from_str::<serde_json::Value>(content) {
                    if evt["type"] == "message_start" {
                        if let Some(it) = evt["message"]["usage"]["input_tokens"].as_u64() {
                            input_tokens = it as u32;
                        }
                    } else if evt["type"] == "content_block_delta" {
                        if let Some(delta) = evt["delta"]["text"].as_str() {
                            if !delta.is_empty() {
                                on_chunk(delta);
                                full_text.push_str(delta);
                            }
                        }
                    } else if evt["type"] == "message_delta" {
                        if let Some(ot) = evt["usage"]["output_tokens"].as_u64() {
                            output_tokens = ot as u32;
                        }
                    }
                }
            }
        }

        if output_tokens == 0 {
            output_tokens = full_text.len() as u32 / 4;
        }

        Ok(AiResponse::new(
            full_text,
            TokenUsage::new(input_tokens, output_tokens),
            self.model.clone(),
        ))
    }

    fn embed(&self, _inputs: &[String]) -> Result<Vec<Vec<f32>>, ForgeError> {
        Err(ForgeError::Ai {
            message: "Anthropic API does not support vector embeddings directly".to_string(),
        })
    }

    fn structured_output(
        &self,
        req: &AiRequest,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, ForgeError> {
        // Enforce JSON output via tool schema pattern on Anthropic
        let endpoint = format!("{}/v1/messages", self.base_url);
        let mut payload = json!({
            "model": self.model,
            "messages": self.prepare_messages(req),
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
            "tools": [
                {
                    "name": "structured_response",
                    "description": "Output the structured analysis according to the schema",
                    "input_schema": schema
                }
            ],
            "tool_choice": { "type": "tool", "name": "structured_response" }
        });

        if !req.system.is_empty() {
            payload["system"] = json!(req.system);
        }

        let response = self
            .agent
            .post(&endpoint)
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", ANTHROPIC_VERSION)
            .set("Content-Type", "application/json")
            .send_json(payload)
            .map_err(|e| map_ureq_error("anthropic", e))?;

        let mut reader = response.into_reader();
        let body_str = read_bounded_body(&mut reader)?;

        let parsed: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| ForgeError::Ai {
                message: format!("failed to parse Anthropic JSON response: {e}"),
            })?;

        if let Some(content_arr) = parsed["content"].as_array() {
            for block in content_arr {
                if block["type"] == "tool_use" && block["name"] == "structured_response" {
                    return Ok(block["input"].clone());
                }
            }
        }

        Err(ForgeError::Ai {
            message: "Anthropic response did not contain expected tool_use structured output"
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AiMessage;
    use std::net::TcpListener;
    use std::thread;

    fn spawn_mock_server<F>(handler: F) -> String
    where
        F: Fn(tiny_http::Request) + Send + 'static,
    {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(l) => l,
            Err(_) => return "http://127.0.0.1:0".to_string(),
        };
        let server = match tiny_http::Server::from_listener(listener, None) {
            Ok(s) => s,
            Err(_) => return "http://127.0.0.1:0".to_string(),
        };
        let addr = match server.server_addr().to_ip() {
            Some(a) => a,
            None => return "http://127.0.0.1:0".to_string(),
        };
        let base_url = format!("http://127.0.0.1:{}", addr.port());

        thread::spawn(move || {
            if let Ok(req) = server.recv() {
                handler(req);
            }
        });

        base_url
    }

    fn json_response(body: String) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
        let mut response = tiny_http::Response::from_string(body);
        if let Ok(h) = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        {
            response.add_header(h);
        }
        response
    }

    #[test]
    fn anthropic_generate_parses_content_and_usage() {
        let base_url = spawn_mock_server(|mut req| {
            let mut body = String::new();
            let _ = req.as_reader().read_to_string(&mut body);
            assert!(body.contains("claude-3-5-sonnet"));

            let resp_json = json!({
                "content": [{ "type": "text", "text": "Claude analysis result" }],
                "usage": { "input_tokens": 15, "output_tokens": 7 },
                "model": "claude-3-5-sonnet-20241022"
            });
            let response = json_response(resp_json.to_string());
            let _ = req.respond(response);
        });

        let provider = AnthropicProvider::new(
            base_url,
            "sk-ant-test-key".to_string(),
            DEFAULT_ANTHROPIC_MODEL.to_string(),
            10,
        );
        assert!(provider.is_ok());
        if let Ok(p) = provider {
            let req = AiRequest::new("System context", vec![AiMessage::user("Analyze")]);
            let res = p.generate(&req);
            assert!(res.is_ok());
            if let Ok(resp) = res {
                assert_eq!(resp.text, "Claude analysis result");
                assert_eq!(resp.usage.input_tokens, 15);
                assert_eq!(resp.usage.output_tokens, 7);
            }
        }
    }

    #[test]
    fn anthropic_embed_returns_unsupported_error() {
        let provider = AnthropicProvider::new(
            "http://127.0.0.1:8080".to_string(),
            "sk-ant-key".to_string(),
            "claude-model".to_string(),
            5,
        );
        assert!(provider.is_ok());
        if let Ok(p) = provider {
            let res = p.embed(&["test".to_string()]);
            assert!(res.is_err());
            if let Err(e) = res {
                assert!(e.to_string().contains("does not support vector embeddings"));
            }
        }
    }
}
