use std::sync::{Arc, Mutex};

use forge_core::error::ForgeError;

use crate::provider::AiProvider;
use crate::types::{AiRequest, AiResponse, TokenUsage};

/// Deterministic in-memory AI provider for unit and integration testing without network calls.
#[derive(Debug, Clone)]
pub struct MockProvider {
    name: &'static str,
    model: String,
    response_text: String,
    structured_json: serde_json::Value,
    embeddings: Vec<Vec<f32>>,
    usage: TokenUsage,
    recorded_requests: Arc<Mutex<Vec<AiRequest>>>,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self {
            name: "mock-provider",
            model: "mock-model-v1".to_string(),
            response_text: "Mock AI generated response".to_string(),
            structured_json: serde_json::json!({ "result": "mock-structured-output" }),
            embeddings: Vec::new(),
            usage: TokenUsage::new(42, 18),
            recorded_requests: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl MockProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_response(mut self, text: impl Into<String>) -> Self {
        self.response_text = text.into();
        self
    }

    pub fn with_structured_output(mut self, value: serde_json::Value) -> Self {
        self.structured_json = value;
        self
    }

    pub fn with_embeddings(mut self, embeddings: Vec<Vec<f32>>) -> Self {
        self.embeddings = embeddings;
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_usage(mut self, usage: TokenUsage) -> Self {
        self.usage = usage;
        self
    }

    pub fn recorded_requests(&self) -> Vec<AiRequest> {
        self.recorded_requests
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn last_request(&self) -> Option<AiRequest> {
        self.recorded_requests
            .lock()
            .ok()
            .and_then(|guard| guard.last().cloned())
    }

    pub fn reset(&self) {
        if let Ok(mut guard) = self.recorded_requests.lock() {
            guard.clear();
        }
    }

    fn record(&self, req: &AiRequest) {
        if let Ok(mut guard) = self.recorded_requests.lock() {
            guard.push(req.clone());
        }
    }
}

impl AiProvider for MockProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    fn generate(&self, req: &AiRequest) -> Result<AiResponse, ForgeError> {
        self.record(req);
        Ok(AiResponse::new(
            self.response_text.clone(),
            self.usage,
            self.model.clone(),
        ))
    }

    fn stream(
        &self,
        req: &AiRequest,
        on_chunk: &mut dyn FnMut(&str),
    ) -> Result<AiResponse, ForgeError> {
        self.record(req);
        for chunk in self.response_text.split_inclusive(' ') {
            on_chunk(chunk);
        }
        Ok(AiResponse::new(
            self.response_text.clone(),
            self.usage,
            self.model.clone(),
        ))
    }

    fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ForgeError> {
        if !self.embeddings.is_empty() {
            return Ok(self.embeddings.clone());
        }
        // Default deterministic 3-dimensional dummy embedding per input
        Ok(inputs
            .iter()
            .enumerate()
            .map(|(idx, _)| vec![idx as f32, (idx + 1) as f32, 0.5])
            .collect())
    }

    fn structured_output(
        &self,
        req: &AiRequest,
        _schema: &serde_json::Value,
    ) -> Result<serde_json::Value, ForgeError> {
        self.record(req);
        Ok(self.structured_json.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AiMessage;

    #[test]
    fn mock_provider_records_requests_and_returns_scripted_outputs() {
        let provider = MockProvider::new()
            .with_response("Hello from mock")
            .with_usage(TokenUsage::new(10, 5))
            .with_model("test-gpt");

        assert_eq!(provider.name(), "mock-provider");

        let req = AiRequest::new(
            "System prompt",
            vec![AiMessage::user("Can you review this?")],
        );
        let resp = provider.generate(&req);
        assert!(resp.is_ok());
        if let Ok(r) = resp {
            assert_eq!(r.text, "Hello from mock");
            assert_eq!(r.usage.input_tokens, 10);
            assert_eq!(r.usage.output_tokens, 5);
            assert_eq!(r.model, "test-gpt");
        }

        assert_eq!(provider.recorded_requests().len(), 1);
        let last = provider.last_request();
        assert!(last.is_some());
        if let Some(r) = last {
            assert_eq!(r.system, "System prompt");
            assert_eq!(r.messages.len(), 1);
        }

        let mut streamed_chunks = Vec::new();
        let stream_resp = provider.stream(&req, &mut |chunk| {
            streamed_chunks.push(chunk.to_string());
        });
        assert!(stream_resp.is_ok());
        assert!(!streamed_chunks.is_empty());
        assert_eq!(streamed_chunks.concat(), "Hello from mock");

        let embeds = provider.embed(&["first".to_string(), "second".to_string()]);
        assert!(embeds.is_ok());
        if let Ok(e) = embeds {
            assert_eq!(e.len(), 2);
        }

        let schema = serde_json::json!({ "type": "object" });
        let structured = provider.structured_output(&req, &schema);
        assert!(structured.is_ok());
        if let Ok(val) = structured {
            assert_eq!(val["result"], "mock-structured-output");
        }
    }
}
