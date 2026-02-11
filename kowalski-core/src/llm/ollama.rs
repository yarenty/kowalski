use super::provider::LLMProvider;
use crate::error::KowalskiError;
use crate::conversation::Message;
use async_trait::async_trait;

pub struct OllamaProvider {
    base_url: String,
}

impl OllamaProvider {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn chat(&self, model: &str, messages: &[Message]) -> Result<String, KowalskiError> {
        // TODO: Implement actual chat logic
        Ok("Ollama response stub".to_string())
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, KowalskiError> {
        // TODO: Implement actual embedding logic
        Ok(vec![0.0; 1536])
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}
