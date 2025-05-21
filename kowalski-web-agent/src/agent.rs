use kowalski_agent_template::TemplateAgent;
use kowalski_core::config::Config;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{ToolInput, TaskType};
use serde_json::json;
use crate::tools::WebTaskType;
use crate::create_web_agent;

/// WebAgent: A specialized agent for web-related tasks
/// This agent is built on top of the TemplateAgent and provides web-specific functionality
pub struct WebAgent {
    template: TemplateAgent,
}

impl WebAgent {
    /// Creates a new WebAgent with the specified configuration
    pub fn new(config: Config) -> Result<Self, KowalskiError> {
        let template = create_web_agent(config)?;
        Ok(Self { template })
    }

    /// Searches the web using the configured search tool
    pub async fn search(&self, query: &str) -> Result<String, KowalskiError> {
        let tool_input = ToolInput::new(
            WebTaskType::Search.name().to_string(),
            query.to_string(),
            json!({})
        );
        let tool_output = self.template.execute_task(tool_input).await?;
        Ok(tool_output.result["results"].as_str().unwrap_or_default().to_string())
    }

    /// Fetches and processes a webpage
    pub async fn fetch_page(&self, url: &str) -> Result<String, KowalskiError> {
        let task_type = if url.contains("twitter.com") || url.contains("linkedin.com") || url.contains("facebook.com") {
            WebTaskType::BrowseDynamic
        } else {
            WebTaskType::ScrapeStatic
        };

        let tool_input = ToolInput::new(
            task_type.name().to_string(),
            url.to_string(),
            json!({})
        );
        let tool_output = self.template.execute_task(tool_input).await?;
        Ok(tool_output.result["content"].as_str().unwrap_or_default().to_string())
    }

    /// Gets the underlying template agent
    pub fn template(&self) -> &TemplateAgent {
        &self.template
    }

    /// Gets a mutable reference to the underlying template agent
    pub fn template_mut(&mut self) -> &mut TemplateAgent {
        &mut self.template
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_web_agent_creation() {
        let config = Config::default();
        let agent = WebAgent::new(config);
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_web_agent_tools() {
        let config = Config::default();
        let agent = WebAgent::new(config).unwrap();
        
        // Test search
        let result = agent.search("test query").await;
        assert!(result.is_ok());

        // Test page fetch
        let result = agent.fetch_page("https://example.com").await;
        assert!(result.is_ok());
    }
} 