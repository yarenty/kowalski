use reqwest::Client;
/// Model: The AI's brain, because apparently we need to give it something to think with.
/// "Models are like students - they learn things but don't always understand them."
///
/// This module provides functionality for managing AI models and their interactions.
/// Think of it as a teacher for your AI, but without the tenure.
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

/// The default model to use when nobody knows what they're doing.
/// "Default models are like default settings - they work until you try to customize them."
#[allow(dead_code)]
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

/// Custom error type for when things go wrong (which they will).
/// "Model errors are like math errors - they're everywhere and they're always your fault."
#[derive(Debug)]
pub enum ModelError {
    Request(reqwest::Error),
    Json(serde_json::Error),
    Server(String),
}

/// Makes the model error printable, because apparently we need to see what went wrong.
/// "Error displays are like warning signs - nobody reads them until it's too late."
impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::Request(e) => write!(f, "Request error: {}", e),
            ModelError::Json(e) => write!(f, "JSON error: {}", e),
            ModelError::Server(e) => write!(f, "Server error: {}", e),
        }
    }
}

/// Makes the model error an actual error, because apparently we need to handle things.
/// "Error implementations are like insurance - you hope you never need them."
impl Error for ModelError {}

impl From<reqwest::Error> for ModelError {
    fn from(err: reqwest::Error) -> Self {
        ModelError::Request(err)
    }
}

impl From<serde_json::Error> for ModelError {
    fn from(err: serde_json::Error) -> Self {
        ModelError::Json(err)
    }
}

/// The main struct that makes our AI models feel special.
/// "Model managers are like librarians - they keep track of everything but nobody appreciates them."
pub struct ModelManager {
    client: Client,
    base_url: String,
}

impl ModelManager {
    /// Creates a new model manager with the specified base URL.
    /// "Creating model managers is like opening a library - it's all fun until someone asks for a book."
    pub fn new(base_url: String) -> Result<Self, ModelError> {
        let client = reqwest::ClientBuilder::new()
            .pool_max_idle_per_host(0)
            .build()
            .map_err(ModelError::Request)?;

        Ok(Self { client, base_url })
    }

    /// Lists available models, because apparently we need to know what we're working with.
    /// "Listing models is like listing your exes - it's longer than you'd like to admit."
    pub async fn list_models(&self) -> Result<ModelsResponse, ModelError> {
        let response = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(ModelError::Server(error_text));
        }

        let models_response: ModelsResponse = response.json().await?;
        Ok(models_response)
    }

    /// Checks if a model exists, because apparently we need to be thorough.
    /// "Checking models is like checking your pockets - you never know what you'll find."
    pub async fn model_exists(&self, model: &str) -> Result<bool, ModelError> {
        let models = self.list_models().await?;
        Ok(models.models.iter().any(|m| m.name == model))
    }

    /// Pulls a model, because apparently we need to download things.
    /// "Pulling models is like pulling teeth - it's painful but necessary."
    pub async fn pull_model(&self, model: &str) -> Result<reqwest::Response, ModelError> {
        let response = self
            .client
            .post(format!("{}/api/pull", self.base_url))
            .json(&serde_json::json!({
                "name": model
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(ModelError::Server(error_text));
        }

        Ok(response)
    }

    /// Deletes a model, because apparently we need to forget things.
    /// "Deleting models is like deleting your browser history - it's a relief until you need it again."
    #[allow(dead_code)]
    pub async fn delete_model(&self, model: &str) -> Result<(), ModelError> {
        let response = self
            .client
            .delete(format!("{}/api/delete", self.base_url))
            .json(&serde_json::json!({
                "name": model
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(ModelError::Server(error_text));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the model manager creation
    /// "Testing model managers is like testing librarians - it's quiet but necessary."
    #[tokio::test]
    async fn test_model_manager_creation() {
        let manager = ModelManager::new("http://localhost:11434".to_string()).unwrap();
        let result = manager.list_models().await;
        assert!(result.is_ok());
    }

    /// Tests the model listing
    /// "Testing model listing is like testing your memory - it's better when it's not empty."
    #[tokio::test]
    async fn test_list_models() {
        let manager = ModelManager::new("http://localhost:11434".to_string()).unwrap();
        let result = manager.list_models().await;
        assert!(result.is_ok());
    }
}
