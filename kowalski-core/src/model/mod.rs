use crate::error::KowalskiError;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// The default model to use when nobody knows what they're doing.
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

/// The main struct that makes our AI models feel special.
pub struct ModelManager {
    client: Client,
    base_url: String,
}

impl ModelManager {
    /// Creates a new model manager with the specified base URL.
    pub fn new(base_url: String) -> Result<Self, KowalskiError> {
        let client = reqwest::ClientBuilder::new()
            .pool_max_idle_per_host(0)
            .build()
            .map_err(KowalskiError::Request)?;

        Ok(Self { client, base_url })
    }

    /// Lists available models
    pub async fn list_models(&self) -> Result<ModelsResponse, KowalskiError> {
        let response = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(KowalskiError::Server(error_text));
        }

        let models_response: ModelsResponse = response.json().await?;
        Ok(models_response)
    }

    /// Checks if a model exists
    pub async fn model_exists(&self, model_name: &str) -> Result<bool, KowalskiError> {
        let models = self.list_models().await?;
        Ok(models.models.iter().any(|m| m.name == model_name))
    }

    /// Pulls a model from the server
    pub async fn pull_model(&self, model_name: &str) -> Result<PullResponse, KowalskiError> {
        let response = self
            .client
            .post(format!("{}/api/pull", self.base_url))
            .json(&serde_json::json!({
                "name": model_name
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(KowalskiError::Server(error_text));
        }

        let pull_response: PullResponse = response.json().await?;
        Ok(pull_response)
    }
}
