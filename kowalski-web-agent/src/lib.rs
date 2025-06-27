pub mod agent;
pub mod config;
pub mod error;
pub mod tools;

pub use agent::WebAgent;
pub use config::WebAgentConfig;
pub use error::WebAgentError;

// Re-export common types
pub use kowalski_core::config::Config;
pub use kowalski_core::error::KowalskiError;
pub use kowalski_core::logging;

use crate::tools::{SearchProvider, SearchTool, WebBrowser, WebScraper, WebTaskType};
use async_trait::async_trait;
use kowalski_agent_template::TemplateAgent;
use kowalski_agent_template::agent::TaskHandler;
use kowalski_core::tools::{ToolInput, ToolOutput};
use serde_json::json;

/// Creates a new web agent with the specified configuration
pub async fn create_web_agent(config: Config) -> Result<TemplateAgent, KowalskiError> {
    let template = TemplateAgent::new(config.clone()).await?;

    // Register tools
    template
        .register_tool(Box::new(SearchTool::new(
            SearchProvider::DuckDuckGo,
            "".to_string(),
        )))
        .await;
    template
        .register_tool(Box::new(WebBrowser::new(config.clone())))
        .await;
    template
        .register_tool(Box::new(WebScraper::new(config)))
        .await;

    // Register task handlers
    struct SearchHandler;
    #[async_trait]
    impl TaskHandler for SearchHandler {
        async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                json!({
                    "result": format!("Searching web: {}", input.content)
                }),
                Some(json!({
                    "handler": "web_search",
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                })),
            ))
        }
    }
    template
        .register_task_handler(WebTaskType::Search, Box::new(SearchHandler))
        .await;

    struct BrowseDynamicHandler;
    #[async_trait]
    impl TaskHandler for BrowseDynamicHandler {
        async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                json!({
                    "result": format!("Browsing dynamic page: {}", input.content)
                }),
                Some(json!({
                    "handler": "browse_dynamic",
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                })),
            ))
        }
    }
    template
        .register_task_handler(WebTaskType::BrowseDynamic, Box::new(BrowseDynamicHandler))
        .await;

    struct ScrapeStaticHandler;
    #[async_trait]
    impl TaskHandler for ScrapeStaticHandler {
        async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                json!({
                    "result": format!("Scraping static page: {}", input.content)
                }),
                Some(json!({
                    "handler": "scrape_static",
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                })),
            ))
        }
    }
    template
        .register_task_handler(WebTaskType::ScrapeStatic, Box::new(ScrapeStaticHandler))
        .await;

    Ok(template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_web_agent() {
        let config = Config::default();
        let agent = create_web_agent(config).await;
        assert!(agent.is_ok());
    }
}
