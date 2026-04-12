use super::provider::LLMProvider;
use crate::agent::types::ChatRequest;
use crate::conversation::Message;
use crate::error::KowalskiError;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

pub struct OllamaProvider {
    base_url: String,
    client: Client,
}

impl OllamaProvider {
    pub fn new(host: &str, port: u16) -> Self {
        let base_url = format!("http://{}:{}", host, port);
        let client = Client::new();
        Self { base_url, client }
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn chat(&self, model: &str, messages: &[Message]) -> Result<String, KowalskiError> {
        let url = format!("{}/api/chat", self.base_url);
        let request = ChatRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            stream: false,    // Force non-streaming for now
            temperature: 0.7, // Default, should be configurable? Passed in method?
            max_tokens: 2048,
            tools: None,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| KowalskiError::Server(format!("Failed to connect to Ollama: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(KowalskiError::Server(format!(
                "Ollama error: {}",
                error_text
            )));
        }

        // Parse non-streaming response
        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| KowalskiError::Server(format!("Failed to parse JSON: {}", e)))?;

        // Ollama non-streaming response structure: { "message": { "content": "..." } }
        let content = response_json["message"]["content"]
            .as_str()
            .ok_or(KowalskiError::Server(
                "No content in Ollama response".to_string(),
            ))?
            .to_string();

        Ok(content)
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, KowalskiError> {
        let url = format!("{}/api/embeddings", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&json!({
                "model": "nomic-embed-text", // Should be configurable
                "prompt": text
            }))
            .send()
            .await
            .map_err(|e| {
                KowalskiError::Memory(format!("Failed to call Ollama embedding: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(KowalskiError::Memory("Ollama embedding failed".to_string()));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| KowalskiError::Memory(format!("Failed to parse embedding JSON: {}", e)))?;

        let embedding = json["embedding"]
            .as_array()
            .ok_or(KowalskiError::Memory(
                "No embedding field in response".to_string(),
            ))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}
