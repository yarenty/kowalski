use crate::builder::AgentBuilder;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput};
use kowalski_tools::document::PdfTool;
use kowalski_tools::web::WebSearchTool;
use serde_json::json;

pub struct GeneralTemplate;

impl GeneralTemplate {
    /// Creates a new general-purpose agent with customizable tools
    pub async fn create_agent(
        tools: Vec<Box<dyn Tool + Send + Sync>>,
        system_prompt: Option<String>,
        temperature: Option<f32>,
    ) -> Result<AgentBuilder, Box<dyn std::error::Error>> {
        let default_prompt = "You are a versatile AI assistant that can help with various tasks.";
        let prompt = system_prompt.unwrap_or_else(|| default_prompt.to_string());
        let temp = temperature.unwrap_or(0.7);

        let builder = AgentBuilder::new()
            .await
            .with_system_prompt(&prompt)
            .with_tools(tools)
            .with_temperature(temp);

        Ok(builder)
    }

    /// Creates a default general-purpose agent with basic tools
    pub async fn create_default_agent() -> Result<AgentBuilder, Box<dyn std::error::Error>> {
        let web_search_tool = WebSearchTool::new("duckduckgo".to_string());
        let pdf_tool = PdfTool;

        let tools = vec![
            Box::new(web_search_tool) as Box<dyn Tool + Send + Sync>,
            Box::new(pdf_tool) as Box<dyn Tool + Send + Sync>,
        ];

        Self::create_agent(tools, None, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kowalski_core::agent::Agent;
    use kowalski_core::error::KowalskiError;

    #[tokio::test]
    async fn test_general_template_default() {
        let builder = GeneralTemplate::create_default_agent().await;
        assert!(builder.is_ok());

        if let Ok(builder) = builder {
            let agent = builder.build().await;
            assert!(agent.is_ok());
        }
    }

    #[tokio::test]
    async fn test_general_template_custom() {
        let web_search_tool = WebSearchTool::new("duckduckgo".to_string());
        let tools = vec![Box::new(web_search_tool) as Box<dyn Tool + Send + Sync>];
        let prompt = "You are a specialized assistant for web research.";

        let builder =
            GeneralTemplate::create_agent(tools, Some(prompt.to_string()), Some(0.5)).await;
        assert!(builder.is_ok());

        if let Ok(builder) = builder {
            let agent = builder.build().await;
            assert!(agent.is_ok());
        }
    }
}
