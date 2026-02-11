use crate::error::KowalskiError;
use crate::conversation::Message;
use async_trait::async_trait;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a chat request to the LLM
    async fn chat(&self, model: &str, messages: &[Message]) -> Result<String, KowalskiError>;
    
    /// Generate embeddings for the given text
    async fn embed(&self, text: &str) -> Result<Vec<f32>, KowalskiError>;
    
    /// Check if the provider supports streaming responses (not used in trait method yet but useful for metadata)
    fn supports_streaming(&self) -> bool;
}
