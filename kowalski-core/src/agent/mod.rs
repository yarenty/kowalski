use crate::agent::types::{ChatRequest, StreamResponse};
use crate::config::Config;
use crate::conversation::Conversation;
use crate::conversation::Message;
use crate::error::KowalskiError;
use crate::role::Role;
use async_trait::async_trait;
use log::info;
use reqwest::Response;
use serde_json;
use std::collections::HashMap;

pub mod types;

/// The core agent trait that all our specialized agents must implement.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Creates a new agent with the specified configuration.
    async fn new(config: Config) -> Result<Self, KowalskiError>
    where
        Self: Sized;

    /// Starts a new conversation
    fn start_conversation(&mut self, model: &str) -> String;

    /// Gets a conversation by ID
    fn get_conversation(&self, id: &str) -> Option<&Conversation>;

    /// Lists all conversations
    fn list_conversations(&self) -> Vec<&Conversation>;

    /// Deletes a conversation
    fn delete_conversation(&mut self, id: &str) -> bool;

    /// Chats with history
    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<Response, KowalskiError>;

    /// Processes a stream response
    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<Message>, KowalskiError>;

    /// Adds a message to a conversation
    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str);

    /// Gets the agent's name
    fn name(&self) -> &str;

    /// Gets the agent's description
    fn description(&self) -> &str;
}

/// The base agent implementation that provides common functionality.
pub struct BaseAgent {
    pub client: reqwest::Client,
    pub config: Config,
    pub conversations: HashMap<String, Conversation>,
    pub name: String,
    pub description: String,
}

impl BaseAgent {
    pub async fn new(config: Config, name: &str, description: &str) -> Result<Self, KowalskiError> {
        let client = reqwest::ClientBuilder::new()
            .pool_max_idle_per_host(0)
            .build()
            .map_err(KowalskiError::Request)?;

        info!("BaseAgent created with name: {}", name);

        Ok(Self {
            client,
            config,
            conversations: HashMap::new(),
            name: name.to_string(),
            description: description.to_string(),
        })
    }
}

#[async_trait]
impl Agent for BaseAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        Self::new(config, "Base Agent", "A basic agent implementation").await
    }

    fn start_conversation(&mut self, model: &str) -> String {
        info!("Starting conversation with model: {}", model);
        let conversation = Conversation::new(model);
        let id = conversation.id.clone();
        self.conversations.insert(id.clone(), conversation);
        id
    }

    fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.conversations.get(id)
    }

    fn list_conversations(&self) -> Vec<&Conversation> {
        self.conversations.values().collect()
    }

    fn delete_conversation(&mut self, id: &str) -> bool {
        self.conversations.remove(id).is_some()
    }

    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<Response, KowalskiError> {
        let conversation = self
            .conversations
            .get_mut(conversation_id)
            .ok_or_else(|| KowalskiError::ConversationNotFound(conversation_id.to_string()))?;

        if let Some(role) = role {
            conversation.add_message("system", &role.get_prompt());

            if let Some(audience) = role.get_audience() {
                conversation.add_message("system", &audience.get_prompt());
            }
            if let Some(preset) = role.get_preset() {
                conversation.add_message("system", &preset.get_prompt());
            }
            if let Some(style) = role.get_style() {
                conversation.add_message("system", &style.get_prompt());
            }
        }

        conversation.add_message("user", content);

        let request = ChatRequest {
            model: conversation.model.clone(),
            messages: conversation.messages.clone(),
            stream: true,
            temperature: self.config.chat.temperature,
            max_tokens: self.config.chat.max_tokens as usize,
            tools: None,
        };

        let response = self
            .client
            .post(format!(
                "http://{}:{}/api/chat",
                self.config.ollama.host, self.config.ollama.port
            ))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(KowalskiError::Server(error_text));
        }

        Ok(response)
    }

    async fn process_stream_response(
        &mut self,
        _conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<Message>, KowalskiError> {
        let text = String::from_utf8(chunk.to_vec())
            .map_err(|e| KowalskiError::Server(format!("Invalid UTF-8: {}", e)))?;

        let stream_response: StreamResponse =
            serde_json::from_str(&text).map_err(|e| KowalskiError::Json(e))?;

        if stream_response.done {
            return Ok(None);
        }

        Ok(Some(stream_response.message))
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        if let Some(conversation) = self.conversations.get_mut(conversation_id) {
            conversation.add_message(role, content);
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }
}
