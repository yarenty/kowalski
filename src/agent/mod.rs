/// Agent module: Because apparently AI needs a personality.
/// "Agents are like cats - they do what they want, when they want." - A Cat Person

mod academic;
mod general;
mod tooling;
mod unified;
pub mod types;

pub use academic::AcademicAgent;
pub use general::GeneralAgent;
pub use tooling::ToolingAgent;
pub use unified::UnifiedAgent;
pub use crate::utils::KowalskiError;
use crate::agent::types::Message;
use async_trait::async_trait;
use crate::config::Config;
use crate::conversation::Conversation;
use crate::role::Role;
use reqwest::Response;
use std::collections::HashMap;
use serde_json;
use log::info;
/// The core agent trait that all our specialized agents must implement.
/// "Traits are like contracts - they're meant to be broken." - A Rust Philosopher
#[async_trait]
#[allow(dead_code)]
pub trait Agent: Send + Sync {
    /// Creates a new agent with the specified configuration.
    /// "Creation is like cooking - sometimes you follow the recipe, sometimes you wing it."
    fn new(config: Config) -> Result<Self, KowalskiError> where Self: Sized;

    /// Starts a new conversation, because silence is overrated.
    fn start_conversation(&mut self, model: &str) -> String;

    /// Gets a conversation by ID, if it hasn't been lost in the void.
    fn get_conversation(&self, id: &str) -> Option<&Conversation>;

    /// Lists all conversations, because sometimes we need to face our past.
    fn list_conversations(&self) -> Vec<&Conversation>;

    /// Deletes a conversation, because sometimes we need to forget.
    fn delete_conversation(&mut self, id: &str) -> bool;

    /// Chats with history, because context matters (sometimes).
    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<Response, KowalskiError>;

    /// Processes a stream response, because we like to watch the AI think.
    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<Message>, KowalskiError>;

    /// Adds a message to a conversation, because monologues are boring.
    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str);

    /// Gets the agent's name, because identity crises are real.
    fn name(&self) -> &str;

    /// Gets the agent's description, because everyone needs a bio.
    fn description(&self) -> &str;
}

/// The base agent implementation that provides common functionality.
/// "Base classes are like parents - they give you structure but let you rebel." - An OOP Therapist
pub struct BaseAgent {
    pub client: reqwest::Client,
    pub config: Config,
    pub conversations: HashMap<String, Conversation>,
    pub name: String,
    pub description: String,
}

impl BaseAgent {
    pub fn new(config: Config, name: &str, description: &str) -> Result<Self, KowalskiError> {
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
    fn new(config: Config) -> Result<Self, KowalskiError> {
        Self::new(config, "Base Agent", "A basic agent implementation")
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
        let conversation = self.conversations.get_mut(conversation_id)
            .ok_or_else(|| KowalskiError::Server("Conversation not found".to_string()))?;

        // Add system messages based on role if provided
        if let Some(role) = role {
            conversation.add_message("system", role.get_prompt());
            
            if let Some(audience) = role.get_audience() {
                conversation.add_message("system", audience.get_prompt());
            }
            if let Some(preset) = role.get_preset() {
                conversation.add_message("system", preset.get_prompt());
            }
            if let Some(style) = role.get_style() {
                conversation.add_message("system", style.get_prompt());
            }
        }

        conversation.add_message("user", content);

        let request = super::agent::types::ChatRequest {
            model: conversation.model.clone(),
            messages: conversation.messages.clone(),
            stream: true,
            temperature: self.config.chat.temperature.unwrap_or(0.7),
            max_tokens: self.config.chat.max_tokens.unwrap_or(2048) as usize,
            tools: None,
        };

        let response = self.client
            .post(format!("{}/api/chat", self.config.ollama.base_url))
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

        let stream_response: super::agent::types::StreamResponse = serde_json::from_str(&text)
            .map_err(|e| KowalskiError::Json(e))?;

        //TODO: Check maybe tool call here?    
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