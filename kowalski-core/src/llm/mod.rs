pub mod ollama;
pub mod openai;
pub mod provider;

pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
pub use provider::{
    ChatOutcome, LLMProvider, TokenStream, ToolDefinition, chat_stream_single_chunk,
    schema_wire_name,
};

use crate::config::Config;
use crate::error::KowalskiError;
use std::sync::Arc;

/// Creates an LLM provider based on the configuration
pub fn create_llm_provider(config: &Config) -> Result<Arc<dyn LLMProvider>, KowalskiError> {
    match config.llm.provider.as_str() {
        "openai" => {
            Ok(Arc::new(
                OpenAIProvider::from_config(&config.llm)?
                    .with_native_tools(config.llm.native_tools)
                    .with_structured_output(config.llm.structured_output),
            ))
        }
        _ => Ok(Arc::new(
            OllamaProvider::new(&config.ollama.host, config.ollama.port)
                .with_native_tools(config.llm.native_tools)
                .with_structured_output(config.llm.structured_output),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::create_llm_provider;
    use crate::config::Config;

    #[test]
    fn openai_headers_are_accepted_and_bad_ones_rejected() {
        let mut c = Config::default();
        c.llm.provider = "openai".to_string();
        c.llm.openai_api_key = Some("test-key".to_string());
        c.llm.openai_headers.insert("X-Org".into(), "my-team".into());
        assert!(create_llm_provider(&c).is_ok());
        c.llm.openai_headers.insert("bad name".into(), "x".into());
        let err = create_llm_provider(&c).err().expect("invalid header name").to_string();
        assert!(err.contains("openai_headers"), "{err}");
    }

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
