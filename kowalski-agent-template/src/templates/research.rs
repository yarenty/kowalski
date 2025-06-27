use crate::builder::AgentBuilder;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput};
use kowalski_tools::web::WebSearchTool;
use kowalski_tools::document::PdfTool;
use serde_json::json;

pub struct ResearchTemplate;

impl ResearchTemplate {
    /// Creates a new research agent with web search and PDF processing capabilities
    pub async fn create_agent() -> Result<AgentBuilder, Box<dyn std::error::Error>> {
        let web_search_tool = WebSearchTool::new("duckduckgo".to_string());
        let pdf_tool = PdfTool;

        let builder = AgentBuilder::new()
            .await.with_system_prompt("You are a research assistant specialized in finding and analyzing academic papers.")
            .with_tool(web_search_tool)
            .with_tool(pdf_tool)
            .with_temperature(0.7);

        Ok(builder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kowalski_core::agent::Agent;
    use kowalski_core::error::KowalskiError;

    #[tokio::test]
    async fn test_research_template() {
        let builder = ResearchTemplate::create_agent().await;
        assert!(builder.is_ok());

        if let Ok(builder) = builder {
            let agent = builder.build().await;
            assert!(agent.is_ok());
        }
    }
}
