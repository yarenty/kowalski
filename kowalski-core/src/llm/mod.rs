pub mod ollama;
pub mod openai;
pub mod provider;

pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
pub use provider::LLMProvider;

use crate::config::Config;
use crate::error::KowalskiError;
use std::sync::Arc;

/// Creates an LLM provider based on the configuration
pub fn create_llm_provider(config: &Config) -> Result<Arc<dyn LLMProvider>, KowalskiError> {
    match config.llm.provider.as_str() {
        "openai" => {
            let api_key = config
                .llm
                .openai_api_key
                .clone()
                .ok_or(KowalskiError::Configuration(
                    "OpenAI API key missing".to_string(),
                ))?;
            Ok(Arc::new(OpenAIProvider::new(&api_key)))
        }
        "ollama" | _ => Ok(Arc::new(OllamaProvider::new(
            &config.ollama.host,
            config.ollama.port,
        ))),
    }
}
