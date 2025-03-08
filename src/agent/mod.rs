/// Agent module: Because apparently AI needs a personality.
/// "Agents are like cats - they do what they want, when they want." - A Cat Person

mod academic;
mod tooling;
mod error;

pub use academic::AcademicAgent;
pub use tooling::ToolingAgent;
pub use error::AgentError;

use async_trait::async_trait;
use crate::config::Config;
use crate::conversation::Conversation;
use crate::role::Role;
use reqwest::Response;
use std::collections::HashMap;

/// The core agent trait that all our specialized agents must implement.
/// "Traits are like contracts - they're meant to be broken." - A Rust Philosopher
#[async_trait]
pub trait Agent: Send + Sync {
    /// Creates a new agent with the specified configuration.
    /// "Creation is like cooking - sometimes you follow the recipe, sometimes you wing it."
    fn new(config: Config) -> Result<Self, AgentError> where Self: Sized;

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
    ) -> Result<Response, AgentError>;

    /// Processes a stream response, because we like to watch the AI think.
    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<String>, AgentError>;

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
    pub fn new(config: Config, name: &str, description: &str) -> Result<Self, AgentError> {
        let client = reqwest::ClientBuilder::new()
            .pool_max_idle_per_host(0)
            .build()
            .map_err(AgentError::RequestError)?;

        Ok(Self {
            client,
            config,
            conversations: HashMap::new(),
            name: name.to_string(),
            description: description.to_string(),
        })
    }
}

// Common implementations that can be reused by concrete agents
impl BaseAgent {
    pub fn start_conversation(&mut self, model: &str) -> String {
        let conversation = Conversation::new(model);
        let id = conversation.id.clone();
        self.conversations.insert(id.clone(), conversation);
        id
    }

    pub fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.conversations.get(id)
    }

    pub fn list_conversations(&self) -> Vec<&Conversation> {
        self.conversations.values().collect()
    }

    pub fn delete_conversation(&mut self, id: &str) -> bool {
        self.conversations.remove(id).is_some()
    }

    pub async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        if let Some(conversation) = self.conversations.get_mut(conversation_id) {
            conversation.add_message(role, content);
        }
    }
} 