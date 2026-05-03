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
    /// Effective HTTP API root (for operator-facing errors).
    api_base_display: String,
}

impl OpenAIProvider {
    /// `api_key` may be empty for some local OpenAI-compatible servers.
    /// `api_base` should be the full OpenAI API root (e.g. `https://api.openai.com/v1` or `http://localhost:1234/v1`).
    pub fn new(api_key: &str, api_base: Option<&str>) -> Self {
        let mut config = OpenAIConfig::new().with_api_key(api_key);
        let api_base_display = if let Some(base) = api_base {
            let trimmed = base.trim();
            if !trimmed.is_empty() {
                config = config.with_api_base(trimmed);
                trimmed.to_string()
            } else {
                "https://api.openai.com/v1".to_string()
            }
        } else {
            "https://api.openai.com/v1".to_string()
        };
        let client = Client::with_config(config);
        Self {
            client,
            embedding_model: "text-embedding-3-small".to_string(),
            api_base_display,
        }
    }

    fn troubleshoot_chat(&self, model: &str, err: impl std::fmt::Display) -> String {
        format!(
            "OpenAI-compatible chat failed (model `{}`, API base `{}`): {}.\n\
             What to check:\n\
             - `config.toml` `[llm]` `provider = \"openai\"` and `openai_api_base` if you use a non-default host (must usually end with `/v1` for OpenAI-compatible HTTP APIs).\n\
             - **API key**: required for `api.openai.com`; many local servers accept an empty or placeholder key.\n\
             - **Model id**: must match the provider (e.g. `gpt-4o-mini`) or your local server’s model list.\n\
             - **Network**: VPN, firewall, corporate proxy, or TLS MITM breaking HTTPS.\n\
             - **Provider logs**: inspect the OpenAI-compatible server console for 4xx/5xx details.",
            model, self.api_base_display, err
        )
    }

    fn troubleshoot_embed(&self, err: impl std::fmt::Display) -> String {
        format!(
            "OpenAI-compatible embeddings failed (model `{}`, API base `{}`): {}.\n\
             What to check:\n\
             - Same connectivity and API key rules as chat.\n\
             - Embedding model id is valid for that provider (default here: `{}`).\n\
             - Local gateways: some require an explicit embeddings route or a different model name.",
            self.embedding_model, self.api_base_display, err, self.embedding_model
        )
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
            .map_err(|e| KowalskiError::Server(self.troubleshoot_chat(model, &e)))?;

        let n_choices = response.choices.len();
        let content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| {
                let finish = response
                    .choices
                    .first()
                    .and_then(|c| c.finish_reason.clone())
                    .map(|r| format!(" first_choice_finish_reason={:?}", r))
                    .unwrap_or_default();
                KowalskiError::Server(format!(
                    "No assistant text in OpenAI-compatible chat response (model `{}`, API base `{}`, {} choice(s){}).\n\
                     What to check: moderation or safety filters, `max_tokens` / empty completion, wrong model id, or a local server returning an unexpected schema.",
                    model, self.api_base_display, n_choices, finish
                ))
            })?;

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
            .map_err(|e| KowalskiError::Memory(self.troubleshoot_embed(&e)))?;

        let n = response.data.len();
        let embedding = response
            .data
            .first()
            .map(|data| data.embedding.clone())
            .ok_or_else(|| {
                KowalskiError::Memory(format!(
                    "No embedding row in OpenAI-compatible response (embedding model `{}`, API base `{}`, {} row(s)).\n\
                     What to check: model supports embeddings on this provider, quota/rate limits, and response schema.",
                    self.embedding_model, self.api_base_display, n
                ))
            })?;

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
        let base = self.api_base_display.clone();
        let model_s = model.to_string();
        Box::pin(async_stream::stream! {
            let mut stream = match client.chat().create_stream(request).await {
                Ok(s) => s,
                Err(e) => {
                    yield Err(KowalskiError::Server(format!(
                        "OpenAI-compatible chat stream failed to start (model `{}`, API base `{}`): {}.\n\
                         What to check: same as non-stream chat — API base, key, model id, and that the server supports streaming for this model.",
                        model_s, base, e
                    )));
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
                            "OpenAI-compatible chat stream chunk error (model `{}`, API base `{}`): {}.\n\
                             What to check: provider timeout, connection drop, or mid-stream API error; retry and inspect server logs.",
                            model_s, base, e
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
