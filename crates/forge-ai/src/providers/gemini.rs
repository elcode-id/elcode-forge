use std::io::{BufRead, BufReader};

use forge_core::error::ForgeError;
use serde_json::json;

use crate::provider::AiProvider;
use crate::providers::http::{create_agent, map_ureq_error, read_bounded_body};
use crate::types::{AiRequest, AiResponse, AiRole, TokenUsage};

pub const DEFAULT_GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com";
pub const DEFAULT_GEMINI_MODEL: &str = "gemini-1.5-pro";
pub const DEFAULT_GEMINI_EMBED_MODEL: &str = "text-embedding-004";

/// Provider adapter for Google Gemini REST API (PRD §19).
#[derive(Debug, Clone)]
pub struct GeminiProvider {
    base_url: String,
    api_key: Option<String>,
    model: String,
    agent: ureq::Agent,
}

impl GeminiProvider {
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

    fn apply_auth(&self, req: ureq::Request) -> ureq::Request {
        if let Some(ref key) = self.api_key {
            req.set("x-goog-api-key", key)
        } else {
            req
        }
    }

    fn prepare_contents(&self, req: &AiRequest) -> Vec<serde_json::Value> {
        let mut contents = Vec::new();
        for msg in &req.messages {
            let role_str = match msg.role {
                AiRole::System | AiRole::User => "user",
                AiRole::Assistant => "model",
            };
            contents.push(json!({
                "role": role_str,
                "parts": [{ "text": msg.content }],
            }));
        }
        if contents.is_empty() {
            contents.push(json!({
                "role": "user",
                "parts": [{ "text": "Analyze the codebase according to instructions." }],
            }));
        }
        contents
    }
}

impl AiProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn generate(&self, req: &AiRequest) -> Result<AiResponse, ForgeError> {
        let endpoint = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, self.model
        );
        let mut payload = json!({
            "contents": self.prepare_contents(req),
            "generationConfig": {
                "temperature": req.temperature,
                "maxOutputTokens": req.max_tokens,
            }
        });

        if !req.system.is_empty() {
            payload["systemInstruction"] = json!({
                "parts": [{ "text": req.system }]
            });
        }

        let ureq_req = self
            .apply_auth(self.agent.post(&endpoint))
            .set("Content-Type", "application/json");

        let response = ureq_req
            .send_json(payload)
            .map_err(|e| map_ureq_error("gemini", e))?;

        let mut reader = response.into_reader();
        let body_str = read_bounded_body(&mut reader)?;

        let parsed: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| ForgeError::Ai {
                message: format!("failed to parse Gemini JSON response: {e}"),
            })?;

        let text = parsed["candidates"]
            .get(0)
            .and_then(|c| c["content"]["parts"].get(0))
            .and_then(|p| p["text"].as_str())
            .unwrap_or("")
            .to_string();

        let prompt_tokens = parsed["usageMetadata"]["promptTokenCount"]
            .as_u64()
            .unwrap_or(0) as u32;
        let candidate_tokens = parsed["usageMetadata"]["candidatesTokenCount"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok(AiResponse::new(
            text,
            TokenUsage::new(prompt_tokens, candidate_tokens),
            self.model.clone(),
        ))
    }

    fn stream(
        &self,
        req: &AiRequest,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<AiResponse, ForgeError> {
        let endpoint = format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
            self.base_url, self.model
        );
        let mut payload = json!({
            "contents": self.prepare_contents(req),
            "generationConfig": {
                "temperature": req.temperature,
                "maxOutputTokens": req.max_tokens,
            }
        });

        if !req.system.is_empty() {
            payload["systemInstruction"] = json!({
                "parts": [{ "text": req.system }]
            });
        }

        let ureq_req = self
            .apply_auth(self.agent.post(&endpoint))
            .set("Content-Type", "application/json");

        let response = ureq_req
            .send_json(payload)
            .map_err(|e| map_ureq_error("gemini", e))?;

        let reader = BufReader::new(response.into_reader());
        let mut full_text = String::new();
        let mut input_tokens = 0u32;
        let mut output_tokens = 0u32;

        for line_res in reader.lines() {
            let line = line_res.map_err(|e| ForgeError::Ai {
                message: format!("error reading Gemini SSE stream line: {e}"),
            })?;

            let trimmed = line.trim();
            if let Some(data) = trimmed.strip_prefix("data:") {
                let content = data.trim();
                if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(content) {
                    if let Some(delta) = chunk["candidates"]
                        .get(0)
                        .and_then(|c| c["content"]["parts"].get(0))
                        .and_then(|p| p["text"].as_str())
                    {
                        if !delta.is_empty() {
                            on_chunk(delta);
                            full_text.push_str(delta);
                        }
                    }
                    if let Some(pt) = chunk["usageMetadata"]["promptTokenCount"].as_u64() {
                        input_tokens = pt as u32;
                    }
                    if let Some(ct) = chunk["usageMetadata"]["candidatesTokenCount"].as_u64() {
                        output_tokens = ct as u32;
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

    fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ForgeError> {
        let embed_model = if self.model.contains("embedding") {
            &self.model
        } else {
            DEFAULT_GEMINI_EMBED_MODEL
        };
        let endpoint = format!(
            "{}/v1beta/models/{}:batchEmbedContents",
            self.base_url, embed_model
        );

        let requests: Vec<_> = inputs
            .iter()
            .map(|text| {
                json!({
                    "model": format!("models/{}", embed_model),
                    "content": { "parts": [{ "text": text }] }
                })
            })
            .collect();

        let payload = json!({ "requests": requests });

        let ureq_req = self
            .apply_auth(self.agent.post(&endpoint))
            .set("Content-Type", "application/json");

        let response = ureq_req
            .send_json(payload)
            .map_err(|e| map_ureq_error("gemini", e))?;

        let mut reader = response.into_reader();
        let body_str = read_bounded_body(&mut reader)?;

        let parsed: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| ForgeError::Ai {
                message: format!("failed to parse Gemini embeddings response: {e}"),
            })?;

        let embeddings_arr = parsed["embeddings"]
            .as_array()
            .ok_or_else(|| ForgeError::Ai {
                message: "missing 'embeddings' array in Gemini response".to_string(),
            })?;

        let mut results = Vec::with_capacity(embeddings_arr.len());
        for item in embeddings_arr {
            let values = item["values"].as_array().ok_or_else(|| ForgeError::Ai {
                message: "missing 'values' in Gemini embedding entry".to_string(),
            })?;

            let mut vec = Vec::with_capacity(values.len());
            for v in values {
                let num = v.as_f64().ok_or_else(|| ForgeError::Ai {
                    message: "invalid non-numeric value in Gemini embedding".to_string(),
                })? as f32;
                vec.push(num);
            }
            results.push(vec);
        }

        Ok(results)
    }

    fn structured_output(
        &self,
        req: &AiRequest,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value, ForgeError> {
        let endpoint = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, self.model
        );
        let mut payload = json!({
            "contents": self.prepare_contents(req),
            "generationConfig": {
                "temperature": req.temperature,
                "maxOutputTokens": req.max_tokens,
                "responseMimeType": "application/json",
                "responseSchema": schema,
            }
        });

        if !req.system.is_empty() {
            payload["systemInstruction"] = json!({
                "parts": [{ "text": req.system }]
            });
        }

        let ureq_req = self
            .apply_auth(self.agent.post(&endpoint))
            .set("Content-Type", "application/json");

        let response = ureq_req
            .send_json(payload)
            .map_err(|e| map_ureq_error("gemini", e))?;

        let mut reader = response.into_reader();
        let body_str = read_bounded_body(&mut reader)?;

        let parsed: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| ForgeError::Ai {
                message: format!("failed to parse Gemini JSON response: {e}"),
            })?;

        let text = parsed["candidates"]
            .get(0)
            .and_then(|c| c["content"]["parts"].get(0))
            .and_then(|p| p["text"].as_str())
            .ok_or_else(|| ForgeError::Ai {
                message: "missing text candidate in Gemini structured response".to_string(),
            })?;

        serde_json::from_str(text).map_err(|e| ForgeError::Ai {
            message: format!("failed to parse Gemini structured JSON text: {e}"),
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
    fn gemini_generate_and_structured_output() {
        let base_url = spawn_mock_server(|mut req| {
            assert!(req.url().contains("gemini-1.5-pro"));
            let mut body = String::new();
            let _ = req.as_reader().read_to_string(&mut body);
            assert!(body.contains("responseSchema"));

            let resp_json = json!({
                "candidates": [{
                    "content": {
                        "parts": [{ "text": "{\"analysis\": \"pass\"}" }]
                    }
                }],
                "usageMetadata": { "promptTokenCount": 20, "candidatesTokenCount": 10 }
            });
            let response = json_response(resp_json.to_string());
            let _ = req.respond(response);
        });

        let provider = GeminiProvider::new(
            base_url,
            Some("fake-gemini-key".to_string()),
            DEFAULT_GEMINI_MODEL.to_string(),
            10,
        );
        assert!(provider.is_ok());
        if let Ok(p) = provider {
            let req = AiRequest::new("System context", vec![AiMessage::user("Analyze")]);
            let schema = json!({ "type": "object" });
            let res = p.structured_output(&req, &schema);
            assert!(res.is_ok());
            if let Ok(val) = res {
                assert_eq!(val["analysis"], "pass");
            }
        }
    }
}
