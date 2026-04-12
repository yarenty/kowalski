use crate::conversation::Message;
use crate::error::KowalskiError;
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;

/// Incremental assistant text from [`LLMProvider::chat_stream`].
pub type TokenStream<'a> = Pin<Box<dyn Stream<Item = Result<String, KowalskiError>> + Send + 'a>>;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a chat request to the LLM
    async fn chat(&self, model: &str, messages: &[Message]) -> Result<String, KowalskiError>;

    /// Generate embeddings for the given text
    async fn embed(&self, text: &str) -> Result<Vec<f32>, KowalskiError>;

    fn supports_streaming(&self) -> bool;

    /// Token deltas (concatenate for the full reply). Empty strings may be omitted by callers.
    fn chat_stream(&self, model: &str, messages: Vec<Message>) -> TokenStream<'_>;
}

/// Single-chunk stream when a provider does not implement native token streaming.
pub fn chat_stream_single_chunk<'a>(
    llm: &'a (dyn LLMProvider + 'a),
    model: &'a str,
    messages: Vec<Message>,
) -> TokenStream<'a> {
    Box::pin(async_stream::stream! {
        match llm.chat(model, &messages).await {
            Ok(t) if !t.is_empty() => yield Ok(t),
            Ok(_) => {}
            Err(e) => yield Err(e),
        }
    })
}
