use serde::{Deserialize, Serialize};
use reqwest::{Client, ClientBuilder};
use std::error::Error;
use std::fmt;

pub const DEFAULT_MODEL: &str = "mistral-small";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
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
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::RequestError(e) => write!(f, "Request error: {}", e),
            AgentError::JsonError(e) => write!(f, "JSON error: {}", e),
            AgentError::ServerError(e) => write!(f, "Server error: {}", e),
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

pub struct Agent {
    client: Client,
    base_url: String,
}

impl Agent {
    pub fn new(base_url: Option<String>) -> Result<Self, AgentError> {
        let client = ClientBuilder::new()
            .pool_max_idle_per_host(0)
            .build()
            .map_err(AgentError::RequestError)?;

        let base_url = base_url.unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

        Ok(Self { client, base_url })
    }

    pub async fn chat(&self, model: &str, messages: Vec<Message>) -> Result<ChatResponse, AgentError> {
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
        };

        let response = self.client
            .post(format!("{}/api/chat", self.base_url))
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
        };

        let response = self.client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        dbg!(&response);
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::ServerError(error_text));
        }

        Ok(response)
    }

    pub async fn list_models(&self) -> Result<ModelsResponse, AgentError> {
        let response = self.client
            .get(format!("{}/api/tags", self.base_url))
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
            .post(format!("{}/api/pull", self.base_url))
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
            .delete(format!("{}/api/delete", self.base_url))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chat() {
        let agent = Agent::new(None).unwrap();
        let messages = vec![
            Message {
                role: "user".to_string(),
                content: "Hello, how are you?".to_string(),
            },
        ];

        let response = agent.chat(DEFAULT_MODEL, messages).await;
        assert!(response.is_ok());
    }
} 