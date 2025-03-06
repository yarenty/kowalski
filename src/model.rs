use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use reqwest::Client;

pub const DEFAULT_MODEL: &str = "mistral-small";

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
pub enum ModelError {
    RequestError(reqwest::Error),
    JsonError(serde_json::Error),
    ServerError(String),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::RequestError(e) => write!(f, "Request error: {}", e),
            ModelError::JsonError(e) => write!(f, "JSON error: {}", e),
            ModelError::ServerError(e) => write!(f, "Server error: {}", e),
        }
    }
}

impl Error for ModelError {}

impl From<reqwest::Error> for ModelError {
    fn from(err: reqwest::Error) -> Self {
        ModelError::RequestError(err)
    }
}

impl From<serde_json::Error> for ModelError {
    fn from(err: serde_json::Error) -> Self {
        ModelError::JsonError(err)
    }
}

pub struct ModelManager {
    client: Client,
    base_url: String,
}

impl ModelManager {
    pub fn new(base_url: String) -> Result<Self, ModelError> {
        let client = reqwest::ClientBuilder::new()
            .pool_max_idle_per_host(0)
            .build()
            .map_err(ModelError::RequestError)?;

        Ok(Self { client, base_url })
    }

    pub async fn list_models(&self) -> Result<ModelsResponse, ModelError> {
        let response = self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(ModelError::ServerError(error_text));
        }

        let models_response: ModelsResponse = response.json().await?;
        Ok(models_response)
    }

    pub async fn pull_model(&self, model: &str) -> Result<reqwest::Response, ModelError> {
        let response = self.client
            .post(format!("{}/api/pull", self.base_url))
            .json(&serde_json::json!({
                "name": model
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(ModelError::ServerError(error_text));
        }

        Ok(response)
    }

    pub async fn delete_model(&self, model: &str) -> Result<(), ModelError> {
        let response = self.client
            .delete(format!("{}/api/delete", self.base_url))
            .json(&serde_json::json!({
                "name": model
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(ModelError::ServerError(error_text));
        }

        Ok(())
    }

    pub async fn model_exists(&self, model: &str) -> Result<bool, ModelError> {
        let models = self.list_models().await?;
        Ok(models.models.iter().any(|m| m.name == model))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_models() {
        let manager = ModelManager::new("http://localhost:11434".to_string()).unwrap();
        let result = manager.list_models().await;
        assert!(result.is_ok());
    }
} 