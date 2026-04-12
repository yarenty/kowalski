use super::provider::{LLMProvider, TokenStream};
use crate::agent::types::ChatRequest;
use crate::conversation::Message;
use crate::error::KowalskiError;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;

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
            stream: false,
            temperature: 0.7,
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

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| KowalskiError::Server(format!("Failed to parse JSON: {}", e)))?;

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
            .json(&serde_json::json!({
                "model": "nomic-embed-text",
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

    fn chat_stream(&self, model: &str, messages: Vec<Message>) -> TokenStream<'_> {
        let url = format!("{}/api/chat", self.base_url);
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: true,
            temperature: 0.7,
            max_tokens: 2048,
            tools: None,
        };
        let client = self.client.clone();
        Box::pin(async_stream::stream! {
            let response = match client.post(&url).json(&request).send().await {
                Ok(r) => r,
                Err(e) => {
                    yield Err(KowalskiError::Server(format!("Ollama stream: {e}")));
                    return;
                }
            };
            if !response.status().is_success() {
                let t = response.text().await.unwrap_or_default();
                yield Err(KowalskiError::Server(format!("Ollama error: {t}")));
                return;
            }
            let mut buf: Vec<u8> = Vec::new();
            let mut bytes_stream = response.bytes_stream();
            while let Some(chunk) = bytes_stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(KowalskiError::Server(format!("Ollama stream read: {e}")));
                        return;
                    }
                };
                buf.extend_from_slice(&chunk);
                while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let raw: Vec<u8> = buf.drain(..=pos).collect();
                    let line = String::from_utf8_lossy(&raw);
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let v: serde_json::Value = match serde_json::from_str(line) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    if let Some(c) = v["message"]["content"].as_str() {
                        if !c.is_empty() {
                            yield Ok(c.to_string());
                        }
                    }
                }
            }
        })
    }
}
