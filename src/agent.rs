/// Agent: The AI's alter ego, because apparently we need to give it a personality.
/// "Agents are like actors - they pretend to be something they're not, but we love them anyway."
///
/// This module provides functionality for managing AI agents and their conversations.
/// Think of it as a therapist for your AI, but without the expensive sessions.
use crate::config::Config;
use crate::conversation::{Conversation, Message};
// use crate::model::{ModelError, ModelManager};
use crate::role::Role;
use reqwest::{Client, ClientBuilder};

use serde::{Deserialize, Serialize};
// use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
// use std::sync::Arc;
// use tokio::sync::Mutex;
// use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub model: String,
    pub response: String,
    pub done: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamResponse {
    pub model: String,
    pub message: Message,
    pub done: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
    pub digest: String,
    pub modified_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PullResponse {
    pub status: String,
    pub digest: Option<String>,
    pub total: Option<u64>,
    pub completed: Option<u64>,
}

/// Custom error type for when things go wrong (which they will).
/// "Errors are like exes - they're everywhere and they're always your fault."
#[derive(Debug)]
pub enum AgentError {
    Request(reqwest::Error),
    Json(serde_json::Error),
    Server(String),
    Config(config::ConfigError),
    Io(std::io::Error),
}

/// Makes the agent error printable, because apparently we need to see what went wrong.
/// "Error displays are like warning signs - nobody reads them until it's too late."
impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::Request(e) => write!(f, "Request error: {}", e),
            AgentError::Json(e) => write!(f, "JSON error: {}", e),
            AgentError::Server(e) => write!(f, "Server error: {}", e),
            AgentError::Config(e) => write!(f, "Config error: {}", e),
            AgentError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

/// Makes the agent error an actual error, because apparently we need to handle things.
/// "Error implementations are like insurance - you hope you never need them."
impl Error for AgentError {}

impl From<reqwest::Error> for AgentError {
    fn from(err: reqwest::Error) -> Self {
        AgentError::Request(err)
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(err: serde_json::Error) -> Self {
        AgentError::Json(err)
    }
}

impl From<config::ConfigError> for AgentError {
    fn from(err: config::ConfigError) -> Self {
        AgentError::Config(err)
    }
}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        AgentError::Io(err)
    }
}

/// The main struct that makes our AI feel special.
/// "Agents are like pets - they're cute but they make a mess."
pub struct Agent {
    client: Client,
    config: Config,
    conversations: HashMap<String, Conversation>,
}

impl Agent {
    /// Creates a new agent with the specified configuration.
    /// "Creating agents is like making a sandwich - simple in theory, complicated in practice."
    pub fn new(config: Config) -> Result<Self, Box<dyn Error>> {
        let client = ClientBuilder::new()
            .pool_max_idle_per_host(0)
            .build()
            .map_err(AgentError::Request)?;

        Ok(Self {
            client,
            config,
            conversations: HashMap::new(),
        })
    }

    /// Starts a new conversation, because apparently we need to track everything.
    /// "Starting conversations is like opening Pandora's box - you never know what you'll get."
    pub fn start_conversation(&mut self, model: &str) -> String {
        let conversation = Conversation::new(model);
        let id = conversation.id.clone();
        self.conversations.insert(id.clone(), conversation);
        id
    }

    /// Gets a conversation by ID, because apparently we need to find things.
    /// "Getting conversations is like finding your keys - they're always in the last place you look."
    pub fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.conversations.get(id)
    }

    /// Lists all conversations, because apparently we need to see them all.
    /// "Listing conversations is like looking at your photo album - it's a trip down memory lane."
    #[allow(dead_code)]
    pub fn list_conversations(&self) -> Vec<&Conversation> {
        self.conversations.values().collect()
    }

    /// Deletes a conversation by ID, because apparently we need to forget things.
    /// "Deleting conversations is like deleting your browser history - it's a relief until you need it again."
    #[allow(dead_code)]
    pub fn delete_conversation(&mut self, id: &str) -> bool {
        self.conversations.remove(id).is_some()
    }

    /// Chats with history, because apparently we need to talk to things.
    /// "Chatting with history is like talking to your old friend - it's a trip down memory lane."
    #[allow(dead_code)]
    pub async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<ChatResponse, AgentError> {
        let conversation = self
            .conversations
            .get_mut(conversation_id)
            .ok_or_else(|| AgentError::Server("Conversation not found".to_string()))?;

        // Add system messages based on role if provided
        if let Some(role) = role {
            conversation.add_message("system", role.get_prompt());

            // Add audience and preset messages if they are part of the translator role
            if let Some(audience) = role.get_audience() {
                conversation.add_message("system", audience.get_prompt());
            }
            if let Some(preset) = role.get_preset() {
                conversation.add_message("system", preset.get_prompt());
            }
            // Add style message if it's part of the illustrator role
            if let Some(style) = role.get_style() {
                conversation.add_message("system", style.get_prompt());
            }
        }

        conversation.add_message("user", content);

        let request = ChatRequest {
            model: conversation.model.clone(),
            messages: conversation.messages.clone(),
            stream: false,
            temperature: self.config.chat.temperature,
            max_tokens: self.config.chat.max_tokens,
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.config.ollama.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::Server(error_text));
        }

        let chat_response: ChatResponse = response.json().await?;
        conversation.add_message("assistant", &chat_response.response);
        Ok(chat_response)
    }

    /// Streams a chat response with history, because apparently we need to be fancy.
    /// "Streaming responses is like watching a movie - it's better when it's not buffering."
    pub async fn stream_chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<reqwest::Response, AgentError> {
        let conversation = self
            .conversations
            .get_mut(conversation_id)
            .ok_or_else(|| AgentError::Server("Conversation not found".to_string()))?;

        // Add system messages based on role if provided
        if let Some(role) = role {
            conversation.add_message("system", role.get_prompt());

            // Add audience and preset messages if they are part of the translator role
            if let Some(audience) = role.get_audience() {
                conversation.add_message("system", audience.get_prompt());
            }
            if let Some(preset) = role.get_preset() {
                conversation.add_message("system", preset.get_prompt());
            }
            // Add style message if it's part of the illustrator role
            if let Some(style) = role.get_style() {
                conversation.add_message("system", style.get_prompt());
            }
        }

        conversation.add_message("user", content);

        let request = ChatRequest {
            model: conversation.model.clone(),
            messages: conversation.messages.clone(),
            stream: true,
            temperature: self.config.chat.temperature,
            max_tokens: self.config.chat.max_tokens,
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.config.ollama.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::Server(error_text));
        }

        Ok(response)
    }

    /// Processes a stream response, because apparently we need to handle things.
    /// "Processing responses is like processing emotions - it's messy but necessary."
    pub async fn process_stream_response(
        &mut self,
        _conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<String>, AgentError> {
        let text = String::from_utf8(chunk.to_vec())
            .map_err(|e| AgentError::Server(format!("Invalid UTF-8: {}", e)))?;

        let stream_response: StreamResponse =
            serde_json::from_str(&text).map_err(AgentError::Json)?;

        if stream_response.done {
            return Ok(None);
        }

        Ok(Some(stream_response.message.content))
    }

    /// Adds a message to a conversation, because apparently we need to remember things.
    /// "Adding messages is like adding ingredients to a recipe - it's all fun until it explodes."
    pub async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        if let Some(conversation) = self.conversations.get_mut(conversation_id) {
            conversation.add_message(role, content);
        }
    }

    /// Chats with a model, because apparently we need to talk to things.
    /// "Chatting with models is like talking to your old friend - it's a trip down memory lane."
    ///
    /// # Arguments
    /// * `model` - The model to chat with
    /// * `messages` - The messages to send to the model
    ///
    /// # Returns
    /// * `Result<ChatResponse, AgentError>` - The response from the model
    ///   
    /// Example:
    /// ```rust
    /// let agent = Agent::new(config).unwrap();
    /// let messages = vec![Message {
    ///     role: "user".to_string(),
    ///     content: "Hello, how are you?".to_string(),
    /// }];
    #[allow(dead_code)]
    pub async fn chat(
        &self,
        model: &str,
        messages: Vec<Message>,
    ) -> Result<ChatResponse, AgentError> {
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
            temperature: self.config.chat.temperature,
            max_tokens: self.config.chat.max_tokens,
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.config.ollama.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::Server(error_text));
        }

        let chat_response: ChatResponse = response.json().await?;
        Ok(chat_response)
    }

    /// Streams a chat response, because apparently we need to be fancy.
    /// "Streaming responses is like watching a movie - it's better when it's not buffering."
    ///
    /// # Arguments
    /// * `model` - The model to chat with
    /// * `messages` - The messages to send to the model
    ///
    /// # Returns
    /// * `Result<reqwest::Response, AgentError>` - The response from the model
    #[allow(dead_code)]
    pub async fn stream_chat(
        &self,
        model: &str,
        messages: Vec<Message>,
    ) -> Result<reqwest::Response, AgentError> {
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: true,
            temperature: self.config.chat.temperature,
            max_tokens: self.config.chat.max_tokens,
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.config.ollama.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::Server(error_text));
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the agent creation
    /// "Testing agents is like testing relationships - it's complicated and usually ends in tears."
    #[tokio::test]
    async fn test_agent_creation() {
        let config = Config::load().unwrap();
        let agent = Agent::new(config).unwrap();
        let messages = vec![Message {
            role: "user".to_string(),
            content: "Hello, how are you?".to_string(),
        }];

        let response = agent.chat("llama2", messages).await;
        assert!(response.is_ok());
    }

    /// Tests the conversation management
    /// "Testing conversations is like testing microphones - it's all about the feedback."
    #[test]
    fn test_conversation_management() {
        // ... existing code ...
    }
}
