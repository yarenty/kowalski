use serde::{Deserialize, Serialize};
use reqwest::{Client, ClientBuilder};
use std::error::Error;
use std::fmt;
use crate::config::Config;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

pub const DEFAULT_MODEL: &str = "mistral-small";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<Message>,
    pub model: String,
    pub created_at: DateTime<Utc>,
}

impl Conversation {
    pub fn new(model: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            messages: Vec::new(),
            model: model.to_string(),
            created_at: Utc::now(),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(Message {
            role: role.to_string(),
            content: content.to_string(),
        });
    }
}

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
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::RequestError(e) => write!(f, "Request error: {}", e),
            AgentError::JsonError(e) => write!(f, "JSON error: {}", e),
            AgentError::ServerError(e) => write!(f, "Server error: {}", e),
            AgentError::ConfigError(e) => write!(f, "Config error: {}", e),
            AgentError::IoError(e) => write!(f, "IO error: {}", e),
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

pub struct Agent {
    client: Client,
    config: Config,
    conversations: HashMap<String, Conversation>,
}

impl Agent {
    pub fn new() -> Result<Self, AgentError> {
        let config = Config::load()?;
        let client = ClientBuilder::new()
            .pool_max_idle_per_host(0)
            .build()
            .map_err(AgentError::RequestError)?;

        Ok(Self { 
            client, 
            config,
            conversations: HashMap::new(),
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

    pub async fn chat_with_history(&mut self, conversation_id: &str, content: &str) -> Result<ChatResponse, AgentError> {
        let conversation = self.conversations.get_mut(conversation_id)
            .ok_or_else(|| AgentError::ServerError("Conversation not found".to_string()))?;

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

    pub async fn stream_chat_with_history(&mut self, conversation_id: &str, content: &str) -> Result<reqwest::Response, AgentError> {
        let conversation = self.conversations.get_mut(conversation_id)
            .ok_or_else(|| AgentError::ServerError("Conversation not found".to_string()))?;

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
        let response = self.client
            .get(format!("{}/api/tags", self.config.ollama.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::ServerError(error_text));
        }

        let models_response: ModelsResponse = response.json().await?;
        Ok(models_response)
    }

    pub async fn pull_model(&self, model: &str) -> Result<reqwest::Response, AgentError> {
        let response = self.client
            .post(format!("{}/api/pull", self.config.ollama.base_url))
            .json(&serde_json::json!({
                "name": model
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::ServerError(error_text));
        }

        Ok(response)
    }

    pub async fn delete_model(&self, model: &str) -> Result<(), AgentError> {
        let response = self.client
            .delete(format!("{}/api/delete", self.config.ollama.base_url))
            .json(&serde_json::json!({
                "name": model
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::ServerError(error_text));
        }

        Ok(())
    }

    pub async fn model_exists(&self, model: &str) -> Result<bool, AgentError> {
        let models = self.list_models().await?;
        Ok(models.models.iter().any(|m| m.name == model))
    }

    pub fn get_default_model(&self) -> &str {
        &self.config.ollama.default_model
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