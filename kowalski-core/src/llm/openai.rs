use super::provider::LLMProvider;
use crate::error::KowalskiError;
use crate::conversation::Message;
use async_trait::async_trait;

pub struct OpenAIProvider {
    api_key: String,
}

impl OpenAIProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat(&self, model: &str, messages: &[Message]) -> Result<String, KowalskiError> {
        // TODO: Implement actual chat logic using async-openai
        Ok("OpenAI response stub".to_string())
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, KowalskiError> {
        // TODO: Implement actual embedding logic
        Ok(vec![0.0; 1536])
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}
