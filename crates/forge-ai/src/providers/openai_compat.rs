use std::io::{BufRead, BufReader};

use forge_core::error::ForgeError;
use serde_json::json;

use crate::provider::AiProvider;
use crate::providers::http::{create_agent, map_ureq_error, read_bounded_body};
use crate::types::{AiRequest, AiResponse, AiRole, TokenUsage};

/// Provider adapter for OpenAI and OpenAI-compatible inference APIs (PRD §19).
///
/// Compatible with:
/// - OpenAI (`https://api.openai.com/v1`)
/// - OpenRouter (`https://openrouter.ai/api/v1`)
/// - Ollama (`http://localhost:11434/v1`)
/// - LM Studio (`http://localhost:1234/v1`)
/// - vLLM, `llama.cpp` server, and any other `/v1/chat/completions`-compliant server.
#[derive(Debug, Clone)]
pub struct OpenAiCompatProvider {
    base_url: String,
    api_key: Option<String>,
    model: String,
    agent: ureq::Agent,
}

impl OpenAiCompatProvider {
    pub fn new(
        base_url: String,
        api_key: Option<String>,
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

        Ok(Self {
            base_url: trimmed_url,
            api_key,
            model,
            agent: create_agent(timeout_secs),
        })
    }

    fn prepare_messages(&self, req: &AiRequest) -> Vec<serde_json::Value> {
        let mut messages = Vec::new();
        if !req.system.is_empty() {
            messages.push(json!({
                "role": "system",
                "content": req.system,
            }));
        }
        for msg in &req.messages {
            let role_str = match msg.role {
                AiRole::System => "system",
                AiRole::User => "user",
                AiRole::Assistant => "assistant",
            };
            messages.push(json!({
                "role": role_str,
                "content": msg.content,
            }));
        }
        messages
    }

    fn apply_auth(&self, req: ureq::Request) -> ureq::Request {
        if let Some(ref key) = self.api_key {
            req.set("Authorization", &format!("Bearer {key}"))
        } else {
            req
        }
    }
}

impl AiProvider for OpenAiCompatProvider {
    fn name(&self) -> &'static str {
        "openai-compatible"
    }

    fn generate(&self, req: &AiRequest) -> Result<AiResponse, ForgeError> {
        let endpoint = format!("{}/chat/completions", self.base_url);
        let payload = json!({
            "model": self.model,
            "messages": self.prepare_messages(req),
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
        });

        let ureq_req = self
            .apply_auth(self.agent.post(&endpoint))
            .set("Content-Type", "application/json");

        let response = ureq_req
            .send_json(payload)
            .map_err(|e| map_ureq_error("openai-compatible", e))?;

        let mut reader = response.into_reader();
        let body_str = read_bounded_body(&mut reader)?;

        let parsed: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| ForgeError::Ai {
                message: format!("failed to parse chat completion JSON response: {e}"),
            })?;

        let text = parsed["choices"]
            .get(0)
            .and_then(|c| c["message"]["content"].as_str())
            .unwrap_or("")
            .to_string();

        let prompt_tokens = parsed["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let completion_tokens = parsed["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;
        let model = parsed["model"].as_str().unwrap_or(&self.model).to_string();

        Ok(AiResponse::new(
            text,
            TokenUsage::new(prompt_tokens, completion_tokens),
            model,
        ))
    }

    fn stream(
        &self,
        req: &AiRequest,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<AiResponse, ForgeError> {
        let endpoint = format!("{}/chat/completions", self.base_url);
        let payload = json!({
            "model": self.model,
            "messages": self.prepare_messages(req),
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
            "stream": true,
        });

        let ureq_req = self
            .apply_auth(self.agent.post(&endpoint))
            .set("Content-Type", "application/json");

        let response = ureq_req
            .send_json(payload)
            .map_err(|e| map_ureq_error("openai-compatible", e))?;

        let reader = BufReader::new(response.into_reader());
        let mut full_text = String::new();
        let mut model_name = self.model.clone();

        for line_res in reader.lines() {
            let line = line_res.map_err(|e| ForgeError::Ai {
                message: format!("error reading SSE stream line: {e}"),
            })?;

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(data) = trimmed.strip_prefix("data:") {
                let content = data.trim();
                if content == "[DONE]" {
                    break;
                }

                if let Ok(chunk_json) = serde_json::from_str::<serde_json::Value>(content) {
                    if let Some(m) = chunk_json["model"].as_str() {
                        model_name = m.to_string();
                    }
                    if let Some(delta) = chunk_json["choices"]
                        .get(0)
                        .and_then(|c| c["delta"]["content"].as_str())
                    {
                        if !delta.is_empty() {
                            on_chunk(delta);
                            full_text.push_str(delta);
                        }
                    }
                }
            }
        }

        // Approximated token counts for streaming response when not explicitly supplied
        let approx_input = req.system.len() as u32 / 4;
        let approx_output = full_text.len() as u32 / 4;

        Ok(AiResponse::new(
            full_text,
            TokenUsage::new(approx_input, approx_output),
            model_name,
        ))
    }

    fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ForgeError> {
        let endpoint = format!("{}/embeddings", self.base_url);
        let payload = json!({
            "model": self.model,
            "input": inputs,
        });

        let ureq_req = self
            .apply_auth(self.agent.post(&endpoint))
            .set("Content-Type", "application/json");

        let response = ureq_req
            .send_json(payload)
            .map_err(|e| map_ureq_error("openai-compatible", e))?;

        let mut reader = response.into_reader();
        let body_str = read_bounded_body(&mut reader)?;

        let parsed: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| ForgeError::Ai {
                message: format!("failed to parse embeddings JSON response: {e}"),
            })?;

        let data_arr = parsed["data"].as_array().ok_or_else(|| ForgeError::Ai {
            message: "invalid embeddings response: missing 'data' array".to_string(),
        })?;

        let mut embeddings = Vec::with_capacity(data_arr.len());
        for item in data_arr {
            let vec_arr = item["embedding"].as_array().ok_or_else(|| ForgeError::Ai {
                message: "missing 'embedding' vector in embedding item".to_string(),
            })?;

            let mut vec_f32 = Vec::with_capacity(vec_arr.len());
            for num in vec_arr {
                let val = num.as_f64().ok_or_else(|| ForgeError::Ai {
                    message: "invalid non-numeric value in embedding vector".to_string(),
                })? as f32;
                vec_f32.push(val);
            }
            embeddings.push(vec_f32);
        }

        Ok(embeddings)
    }

    fn structured_output(
        &self,
        req: &AiRequest,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, ForgeError> {
        let endpoint = format!("{}/chat/completions", self.base_url);
        let payload = json!({
            "model": self.model,
            "messages": self.prepare_messages(req),
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "structured_response",
                    "strict": true,
                    "schema": schema,
                }
            }
        });

        let ureq_req = self
            .apply_auth(self.agent.post(&endpoint))
            .set("Content-Type", "application/json");

        let response = ureq_req
            .send_json(payload)
            .map_err(|e| map_ureq_error("openai-compatible", e))?;

        let mut reader = response.into_reader();
        let body_str = read_bounded_body(&mut reader)?;

        let parsed: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| ForgeError::Ai {
                message: format!("failed to parse chat completion JSON: {e}"),
            })?;

        let content_str = parsed["choices"]
            .get(0)
            .and_then(|c| c["message"]["content"].as_str())
            .ok_or_else(|| ForgeError::Ai {
                message: "missing message content in structured output response".to_string(),
            })?;

        serde_json::from_str(content_str).map_err(|e| ForgeError::Ai {
            message: format!("structured output text is not valid JSON conforming to schema: {e}"),
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
    fn generate_parses_json_response_and_token_usage() {
        let base_url = spawn_mock_server(|mut req| {
            let mut body = String::new();
            let _ = req.as_reader().read_to_string(&mut body);
            assert!(body.contains("gpt-4o-mini"));

            let resp_json = json!({
                "choices": [{
                    "message": { "role": "assistant", "content": "Analysis completed successfully" }
                }],
                "usage": { "prompt_tokens": 12, "completion_tokens": 8 },
                "model": "gpt-4o-mini"
            });
            let response = json_response(resp_json.to_string());
            let _ = req.respond(response);
        });

        let provider = OpenAiCompatProvider::new(
            base_url,
            Some("sk-test-secret".to_string()),
            "gpt-4o-mini".to_string(),
            10,
        );
        assert!(provider.is_ok());
        if let Ok(p) = provider {
            let req = AiRequest::new("System instructions", vec![AiMessage::user("Hello")]);
            let res = p.generate(&req);
            assert!(res.is_ok());
            if let Ok(resp) = res {
                assert_eq!(resp.text, "Analysis completed successfully");
                assert_eq!(resp.usage.input_tokens, 12);
                assert_eq!(resp.usage.output_tokens, 8);
                assert_eq!(resp.model, "gpt-4o-mini");
            }
        }
    }

    #[test]
    fn stream_parses_sse_chunks_until_done() {
        let base_url = spawn_mock_server(|req| {
            let sse_body = "data: {\"choices\":[{\"delta\":{\"content\":\"Chunk1 \"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"Chunk2\"}}]}\n\ndata: [DONE]\n\n";
            let mut response = tiny_http::Response::from_string(sse_body);
            if let Ok(h) =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/event-stream"[..])
            {
                response.add_header(h);
            }
            let _ = req.respond(response);
        });

        let provider = OpenAiCompatProvider::new(base_url, None, "ollama-local".to_string(), 10);
        assert!(provider.is_ok());
        if let Ok(p) = provider {
            let req = AiRequest::new("Sys", vec![AiMessage::user("Stream me")]);
            let mut chunks = Vec::new();
            let res = p.stream(&req, &mut |chunk| chunks.push(chunk.to_string()));
            assert!(res.is_ok());
            if let Ok(resp) = res {
                assert_eq!(resp.text, "Chunk1 Chunk2");
                assert_eq!(chunks, vec!["Chunk1 ", "Chunk2"]);
            }
        }
    }

    #[test]
    fn embed_parses_vectors() {
        let base_url = spawn_mock_server(|req| {
            let resp_json = json!({
                "data": [
                    { "embedding": [0.1, 0.2, 0.3] },
                    { "embedding": [0.4, 0.5, 0.6] }
                ]
            });
            let response = json_response(resp_json.to_string());
            let _ = req.respond(response);
        });

        let provider =
            OpenAiCompatProvider::new(base_url, None, "text-embedding-3-small".to_string(), 10);
        assert!(provider.is_ok());
        if let Ok(p) = provider {
            let res = p.embed(&["a".to_string(), "b".to_string()]);
            assert!(res.is_ok());
            if let Ok(vecs) = res {
                assert_eq!(vecs.len(), 2);
                assert_eq!(vecs[0], vec![0.1, 0.2, 0.3]);
                assert_eq!(vecs[1], vec![0.4, 0.5, 0.6]);
            }
        }
    }

    #[test]
    fn structured_output_parses_json_content() {
        let base_url = spawn_mock_server(|req| {
            let resp_json = json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "{\"status\":\"success\",\"risk\":\"low\"}"
                    }
                }]
            });
            let response = json_response(resp_json.to_string());
            let _ = req.respond(response);
        });

        let provider = OpenAiCompatProvider::new(base_url, None, "gpt-4o".to_string(), 10);
        assert!(provider.is_ok());
        if let Ok(p) = provider {
            let req = AiRequest::new("Sys", vec![AiMessage::user("Check")]);
            let schema = json!({ "type": "object" });
            let res = p.structured_output(&req, &schema);
            assert!(res.is_ok());
            if let Ok(val) = res {
                assert_eq!(val["status"], "success");
                assert_eq!(val["risk"], "low");
            }
        }
    }

    #[test]
    fn http_error_maps_cleanly_without_echoing_api_key() {
        let base_url = spawn_mock_server(|req| {
            let response =
                tiny_http::Response::from_string("Unauthorized access").with_status_code(401);
            let _ = req.respond(response);
        });

        let provider = OpenAiCompatProvider::new(
            base_url,
            Some("sk-secret-token-never-leak".to_string()),
            "gpt-4o".to_string(),
            10,
        );
        assert!(provider.is_ok());
        if let Ok(p) = provider {
            let req = AiRequest::new("Sys", vec![AiMessage::user("Ping")]);
            let res = p.generate(&req);
            assert!(res.is_err());
            if let Err(e) = res {
                let err_str = e.to_string();
                assert!(err_str.contains("HTTP 401"));
                assert!(!err_str.contains("sk-secret-token-never-leak"));
            }
        }
    }
}
