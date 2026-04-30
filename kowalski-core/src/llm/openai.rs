use super::provider::LLMProvider;
use super::provider::TokenStream;
use crate::conversation::Message;
use crate::error::KowalskiError;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        chat::{
            ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
            ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
            CreateChatCompletionRequestArgs,
        },
        embeddings::CreateEmbeddingRequestArgs,
    },
};
use async_trait::async_trait;
use futures::StreamExt;

pub struct OpenAIProvider {
    client: Client<OpenAIConfig>,
    embedding_model: String,
}

impl OpenAIProvider {
    /// `api_key` may be empty for some local OpenAI-compatible servers.
    /// `api_base` should be the full OpenAI API root (e.g. `https://api.openai.com/v1` or `http://localhost:1234/v1`).
    pub fn new(api_key: &str, api_base: Option<&str>) -> Self {
        let mut config = OpenAIConfig::new().with_api_key(api_key);
        if let Some(base) = api_base {
            let trimmed = base.trim();
            if !trimmed.is_empty() {
                config = config.with_api_base(trimmed);
            }
        }
        let client = Client::with_config(config);
        Self {
            client,
            embedding_model: "text-embedding-3-small".to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat(&self, model: &str, messages: &[Message]) -> Result<String, KowalskiError> {
        let openai_messages = messages_to_openai(messages)?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(openai_messages)
            .build()
            .map_err(|e| KowalskiError::Initialization(format!("OpenAI request error: {}", e)))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| KowalskiError::Server(format!("OpenAI API error: {}", e)))?;

        let content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or(KowalskiError::Server(
                "No content in OpenAI response".to_string(),
            ))?;

        Ok(content)
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, KowalskiError> {
        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.embedding_model)
            .input(text)
            .build()
            .map_err(|e| KowalskiError::Initialization(format!("OpenAI embedding error: {}", e)))?;

        let response = self
            .client
            .embeddings()
            .create(request)
            .await
            .map_err(|e| KowalskiError::Memory(format!("OpenAI embedding API error: {}", e)))?;

        let embedding = response
            .data
            .first()
            .map(|data| data.embedding.clone())
            .ok_or(KowalskiError::Memory(
                "No embedding in OpenAI response".to_string(),
            ))?;

        Ok(embedding)
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn chat_stream(&self, model: &str, messages: Vec<Message>) -> TokenStream<'_> {
        let openai_messages = match messages_to_openai(&messages) {
            Ok(m) => m,
            Err(e) => {
                return Box::pin(futures::stream::once(async move { Err(e) }));
            }
        };
        let request = match CreateChatCompletionRequestArgs::default()
            .model(model.to_string())
            .messages(openai_messages)
            .stream(true)
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                return Box::pin(futures::stream::once(async move {
                    Err(KowalskiError::Initialization(format!(
                        "OpenAI stream request: {e}"
                    )))
                }));
            }
        };
        let client = self.client.clone();
        Box::pin(async_stream::stream! {
            let mut stream = match client.chat().create_stream(request).await {
                Ok(s) => s,
                Err(e) => {
                    yield Err(KowalskiError::Server(format!("OpenAI stream: {e}")));
                    return;
                }
            };
            while let Some(item) = stream.next().await {
                match item {
                    Ok(resp) => {
                        for choice in resp.choices {
                            if let Some(ref c) = choice.delta.content
                                && !c.is_empty() {
                                    yield Ok(c.clone());
                                }
                        }
                    }
                    Err(e) => {
                        yield Err(KowalskiError::Server(format!(
                            "OpenAI stream chunk: {e}"
                        )));
                        return;
                    }
                }
            }
        })
    }
}

fn messages_to_openai(
    messages: &[Message],
) -> Result<Vec<ChatCompletionRequestMessage>, KowalskiError> {
    let mut openai_messages: Vec<ChatCompletionRequestMessage> = Vec::new();

    for msg in messages {
        match msg.role.as_str() {
            "system" => {
                openai_messages.push(ChatCompletionRequestMessage::System(
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(msg.content.clone())
                        .build()
                        .map_err(|e| {
                            KowalskiError::Initialization(format!("OpenAI message error: {}", e))
                        })?,
                ));
            }
            "user" => {
                openai_messages.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(msg.content.clone())
                        .build()
                        .map_err(|e| {
                            KowalskiError::Initialization(format!("OpenAI message error: {}", e))
                        })?,
                ));
            }
            "assistant" => {
                openai_messages.push(ChatCompletionRequestMessage::Assistant(
                    ChatCompletionRequestAssistantMessageArgs::default()
                        .content(msg.content.clone())
                        .build()
                        .map_err(|e| {
                            KowalskiError::Initialization(format!("OpenAI message error: {}", e))
                        })?,
                ));
            }
            _ => {
                openai_messages.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(format!("[{}] {}", msg.role, msg.content))
                        .build()
                        .map_err(|e| {
                            KowalskiError::Initialization(format!("OpenAI message error: {}", e))
                        })?,
                ));
            }
        }
    }
    Ok(openai_messages)
}
