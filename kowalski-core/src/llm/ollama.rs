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

    fn troubleshoot_connect(&self, endpoint: &str, err: &reqwest::Error) -> String {
        format!(
            "Cannot reach Ollama at {} (requested {}): {}.\n\
             What to check:\n\
             - Is the Ollama daemon running? (`ollama serve`, or start the Ollama app.)\n\
             - Does Kowalski `config.toml` `[ollama]` host/port match where Ollama listens? (default http://127.0.0.1:11434)\n\
             - From the same machine: `curl -s {}/api/tags` should return JSON, not \"connection refused\".\n\
             - If you use a remote Ollama, confirm firewall/VPN and that `OLLAMA_HOST` on the Ollama side allows your client.",
            self.base_url, endpoint, err, self.base_url
        )
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
            .map_err(|e| KowalskiError::Server(self.troubleshoot_connect(&url, &e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(KowalskiError::Server(format!(
                "Ollama returned HTTP {} from {} for model `{}`. Body: {}.\n\
                 What to check:\n\
                 - Model pulled? `ollama pull {}`\n\
                 - Ollama logs for stack traces (terminal where `ollama serve` runs).",
                status, url, model, error_text.trim(), model
            )));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| {
                KowalskiError::Server(format!(
                    "Ollama returned success HTTP but invalid JSON from {}: {}. Raw response may be truncated in logs.",
                    url, e
                ))
            })?;

        let content = response_json["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                KowalskiError::Server(format!(
                    "No `message.content` in Ollama JSON from {}. Keys present: {:?}. Full body (trimmed): {:.500}",
                    url,
                    response_json
                        .as_object()
                        .map(|o| o.keys().cloned().collect::<Vec<_>>())
                        .unwrap_or_default(),
                    response_json.to_string()
                ))
            })?
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
            .map_err(|e| KowalskiError::Memory(self.troubleshoot_connect(&url, &e)))?;

        let status = response.status();
        if !status.is_success() {
            let t = response.text().await.unwrap_or_default();
            return Err(KowalskiError::Memory(format!(
                "Ollama embedding HTTP {} from {}: {}",
                status.as_u16(),
                url,
                t.trim()
            )));
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
        let base_url = self.base_url.clone();
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
                    yield Err(KowalskiError::Server(format!(
                        "Cannot reach Ollama at {} (stream {}): {}.\n\
                         What to check: same as non-stream — run `ollama serve`, match `[ollama]` in config.toml, `curl -s {}/api/tags`.",
                        base_url, url, e, base_url
                    )));
                    return;
                }
            };
            let status = response.status();
            if !status.is_success() {
                let t = response.text().await.unwrap_or_default();
                yield Err(KowalskiError::Server(format!(
                    "Ollama stream returned HTTP {} from {}: {}",
                    status.as_u16(),
                    url,
                    t.trim()
                )));
                return;
            }
            let mut buf: Vec<u8> = Vec::new();
            let mut bytes_stream = response.bytes_stream();
            while let Some(chunk) = bytes_stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(KowalskiError::Server(format!(
                            "Ollama stream read error from {}: {}",
                            url, e
                        )));
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
                    if let Some(c) = v["message"]["content"].as_str()
                        && !c.is_empty() {
                            yield Ok(c.to_string());
                        }
                }
            }
        })
    }
}
