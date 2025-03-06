use serde::{Deserialize, Serialize};
use reqwest::{Client, ClientBuilder};
use std::error::Error;
use std::fmt;
use crate::config::Config;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::audience::Audience;
use crate::preset::Preset;
use crate::role::Role;
use crate::style::Style;
use crate::conversation::{Conversation, Message};
use crate::model::{ModelManager, ModelError, DEFAULT_MODEL};

pub const DEFAULT_MODEL: &str = "mistral-small";

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

#[derive(Debug)]
pub enum AgentError {
    RequestError(reqwest::Error),
    JsonError(serde_json::Error),
    ServerError(String),
    ConfigError(config::ConfigError),
    IoError(std::io::Error),
    ModelError(ModelError),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::RequestError(e) => write!(f, "Request error: {}", e),
            AgentError::JsonError(e) => write!(f, "JSON error: {}", e),
            AgentError::ServerError(e) => write!(f, "Server error: {}", e),
            AgentError::ConfigError(e) => write!(f, "Config error: {}", e),
            AgentError::IoError(e) => write!(f, "IO error: {}", e),
            AgentError::ModelError(e) => write!(f, "Model error: {}", e),
        }
    }
}

impl Error for AgentError {}

impl From<reqwest::Error> for AgentError {
    fn from(err: reqwest::Error) -> Self {
        AgentError::RequestError(err)
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(err: serde_json::Error) -> Self {
        AgentError::JsonError(err)
    }
}

impl From<config::ConfigError> for AgentError {
    fn from(err: config::ConfigError) -> Self {
        AgentError::ConfigError(err)
    }
}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        AgentError::IoError(err)
    }
}

impl From<ModelError> for AgentError {
    fn from(err: ModelError) -> Self {
        AgentError::ModelError(err)
    }
}

pub struct Agent {
    client: Client,
    config: Config,
    conversations: HashMap<String, Conversation>,
    model_manager: ModelManager,
}

impl Agent {
    pub fn new() -> Result<Self, AgentError> {
        let config = Config::load()?;
        let client = ClientBuilder::new()
            .pool_max_idle_per_host(0)
            .build()
            .map_err(AgentError::RequestError)?;
        
        let model_manager = ModelManager::new(config.ollama.base_url.clone())?;

        Ok(Self { 
            client, 
            config,
            conversations: HashMap::new(),
            model_manager,
        })
    }

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

    pub async fn chat_with_history(&mut self, conversation_id: &str, content: &str, role: Option<Role>) -> Result<ChatResponse, AgentError> {
        let conversation = self.conversations.get_mut(conversation_id)
            .ok_or_else(|| AgentError::ServerError("Conversation not found".to_string()))?;

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

        let response = self.client
            .post(format!("{}/api/chat", self.config.ollama.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::ServerError(error_text));
        }

        let chat_response: ChatResponse = response.json().await?;
        conversation.add_message("assistant", &chat_response.response);
        Ok(chat_response)
    }

    pub async fn stream_chat_with_history(&mut self, conversation_id: &str, content: &str, role: Option<Role>) -> Result<reqwest::Response, AgentError> {
        let conversation = self.conversations.get_mut(conversation_id)
            .ok_or_else(|| AgentError::ServerError("Conversation not found".to_string()))?;

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

        let response = self.client
            .post(format!("{}/api/chat", self.config.ollama.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::ServerError(error_text));
        }

        Ok(response)
    }

    pub async fn process_stream_response(&mut self, conversation_id: &str, chunk: &[u8]) -> Result<Option<String>, AgentError> {
        let text = String::from_utf8(chunk.to_vec())
            .map_err(|e| AgentError::ServerError(format!("Invalid UTF-8: {}", e)))?;

        let stream_response: StreamResponse = serde_json::from_str(&text)
            .map_err(|e| AgentError::JsonError(e))?;

        if stream_response.done {
            //TODO: Add the final message to the conversation:
            // this is wrong as final message here is just empty
            // temporary solution is to add the content to the buffer -on main.rs
            // but that is just ... not right!!
            // have no time at them moment to fix this
            // TODO: fix this
            // if let Some(conversation) = self.conversations.get_mut(conversation_id) {
            //     conversation.add_message("assistant", &stream_response.message.content);
            // }
            return Ok(None);
        }

        // For streaming responses, we only return the new content
        // The final message will be added when done is true
        Ok(Some(stream_response.message.content))
    }

    pub async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        if let Some(conversation) = self.conversations.get_mut(conversation_id) {
            conversation.add_message(role, content);
        }
    }

    pub async fn chat(&self, model: &str, messages: Vec<Message>) -> Result<ChatResponse, AgentError> {
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
            temperature: self.config.chat.temperature,
            max_tokens: self.config.chat.max_tokens,
        };

        let response = self.client
            .post(format!("{}/api/chat", self.config.ollama.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::ServerError(error_text));
        }

        let chat_response: ChatResponse = response.json().await?;
        Ok(chat_response)
    }

    pub async fn stream_chat(&self, model: &str, messages: Vec<Message>) -> Result<reqwest::Response, AgentError> {
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: true,
            temperature: self.config.chat.temperature,
            max_tokens: self.config.chat.max_tokens,
        };

        let response = self.client
            .post(format!("{}/api/chat", self.config.ollama.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::ServerError(error_text));
        }

        Ok(response)
    }

    pub async fn list_models(&self) -> Result<ModelsResponse, AgentError> {
        Ok(self.model_manager.list_models().await?)
    }

    pub async fn pull_model(&self, model: &str) -> Result<reqwest::Response, AgentError> {
        Ok(self.model_manager.pull_model(model).await?)
    }

    pub async fn delete_model(&self, model: &str) -> Result<(), AgentError> {
        Ok(self.model_manager.delete_model(model).await?)
    }

    pub async fn model_exists(&self, model: &str) -> Result<bool, AgentError> {
        Ok(self.model_manager.model_exists(model).await?)
    }

    pub fn get_default_model(&self) -> &str {
        DEFAULT_MODEL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chat() {
        let agent = Agent::new().unwrap();
        let messages = vec![
            Message {
                role: "user".to_string(),
                content: "Hello, how are you?".to_string(),
            },
        ];

        let response = agent.chat(agent.get_default_model(), messages).await;
        assert!(response.is_ok());
    }
} 