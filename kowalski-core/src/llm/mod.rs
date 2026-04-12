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
                .unwrap_or_default();
            let base = config.llm.openai_api_base.as_deref();
            Ok(Arc::new(OpenAIProvider::new(&api_key, base)))
        }
        "ollama" | _ => Ok(Arc::new(OllamaProvider::new(
            &config.ollama.host,
            config.ollama.port,
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::create_llm_provider;
    use crate::config::Config;

    #[test]
    fn openai_compatible_provider_with_custom_base() {
        let mut c = Config::default();
        c.llm.provider = "openai".to_string();
        c.llm.openai_api_key = Some("test-key".to_string());
        c.llm.openai_api_base = Some("https://api.example.com/v1".to_string());
        assert!(create_llm_provider(&c).is_ok());
    }

    #[test]
    fn openai_compatible_accepts_empty_key_for_local_servers() {
        let mut c = Config::default();
        c.llm.provider = "openai".to_string();
        c.llm.openai_api_key = Some(String::new());
        c.llm.openai_api_base = Some("http://127.0.0.1:1234/v1".to_string());
        assert!(create_llm_provider(&c).is_ok());
    }
}
