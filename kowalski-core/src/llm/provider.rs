//! LLM abstraction used by [`crate::agent::BaseAgent`].
//!
//! ## Operator-facing errors (convention for implementors)
//!
//! Callers (including the HTTP `/api/chat` path) surface `KowalskiError` strings to operators and
//! CLI tools. When a request fails, prefer **actionable** messages over bare library errors:
//!
//! - **Include context**: which **API base / host**, **model id**, and **operation** (chat, embed,
//!   stream).
//! - **Add a short “What to check” list**: daemon or process up? `config.toml` `[llm]` keys
//!   (`provider`, `openai_api_base`, API key, Ollama host/port)? Network / VPN / TLS? Model
//!   spelling and `pull` / provider catalog?
//! - **Map HTTP / API bodies** when available (status + trimmed body), not only `Display` of the
//!   client error.
//!
//! Reference implementations: [`super::ollama::OllamaProvider`], [`super::openai::OpenAIProvider`].

use crate::conversation::Message;
use crate::error::KowalskiError;
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;

/// Incremental assistant text from [`LLMProvider::chat_stream`].
pub type TokenStream<'a> = Pin<Box<dyn Stream<Item = Result<String, KowalskiError>> + Send + 'a>>;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a chat request to the LLM.
    ///
    /// On failure, return [`KowalskiError::Server`] with an operator-oriented message (see module
    /// docs above).
    async fn chat(&self, model: &str, messages: &[Message]) -> Result<String, KowalskiError>;

    /// Generate embeddings for the given text.
    ///
    /// On failure, return [`KowalskiError::Memory`] with an operator-oriented message.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, KowalskiError>;

    fn supports_streaming(&self) -> bool;

    /// Token deltas (concatenate for the full reply). Empty strings may be omitted by callers.
    ///
    /// Stream errors should follow the same clarity convention as [`LLMProvider::chat`].
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
