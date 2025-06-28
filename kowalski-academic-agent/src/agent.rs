use crate::config::AcademicAgentConfig;
use async_trait::async_trait;
use kowalski_agent_template::TemplateAgent;
use kowalski_agent_template::templates::general::GeneralTemplate;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::Conversation;
use kowalski_core::error::KowalskiError;
use kowalski_core::role::Role;
use kowalski_core::tools::{Tool, ToolInput};
use kowalski_tools::document::PdfTool;
use kowalski_tools::web::WebSearchTool;
use serde::{Deserialize, Serialize};

/// AcademicAgent: A specialized agent for academic tasks
/// This agent is built on top of the TemplateAgent and provides academic-specific functionality
#[allow(dead_code)]
pub struct AcademicAgent {
    agent: TemplateAgent,
    config: AcademicAgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSearchResult {
    pub title: String,
    pub authors: String,
    pub abstract_text: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationResult {
    pub citation: String,
    pub format: String,
}

impl AcademicAgent {
    /// Creates a new AcademicAgent with the specified configuration
    pub async fn new(_config: Config) -> Result<Self, KowalskiError> {
        // TODO: Convert Config to AcademicAgentConfig if needed
        let academic_config = AcademicAgentConfig::default();
        let search_tool = WebSearchTool::new("duckduckgo".to_string());
        let pdf_tool = PdfTool;
        let tools: Vec<Box<dyn Tool + Send + Sync>> =
            vec![Box::new(search_tool), Box::new(pdf_tool)];
        let builder = GeneralTemplate::create_agent(
            tools,
            Some("You are an academic research assistant specialized in finding and analyzing academic papers. You have access to web_search and pdf tools. Use them to answer questions about academic research.".to_string()),
            Some(0.7),
        )
        .await
        .map_err(|e| KowalskiError::Configuration(e.to_string()))?;
        let agent = builder.build().await?;
        Ok(Self {
            agent,
            config: academic_config,
        })
    }

    /// Searches for academic papers
    pub async fn search_papers(
        &self,
        query: &str,
    ) -> Result<Vec<PaperSearchResult>, KowalskiError> {
        let mut tools = self.agent.tool_chain.write().await;
        let tool = tools.iter_mut().find(|t| t.name() == "web_search");
        let tool = match tool {
            Some(t) => t,
            None => {
                return Err(KowalskiError::ToolExecution(
                    "web_search tool not found".to_string(),
                ));
            }
        };
        let input = ToolInput::new(
            "search".to_string(),
            query.to_string(),
            serde_json::json!({"query": query}),
        );
        let _output = tool.execute(input).await?;
        // For now, return a simple result since DuckDuckGo doesn't provide academic-specific results
        Ok(vec![PaperSearchResult {
            title: query.to_string(),
            authors: "Unknown".to_string(),
            abstract_text:
                "Academic search results would be available with a proper academic search API."
                    .to_string(),
            url: "".to_string(),
        }])
    }

    /// Generates a citation for a reference
    pub async fn generate_citation(
        &self,
        reference: &str,
    ) -> Result<CitationResult, KowalskiError> {
        // For now, return a simple citation format
        Ok(CitationResult {
            citation: format!("Citation for: {}", reference),
            format: "APA".to_string(),
        })
    }

    /// Parses and analyzes an academic paper
    pub async fn parse_paper(&self, content: &str) -> Result<String, KowalskiError> {
        let mut tools = self.agent.tool_chain.write().await;
        let tool = tools.iter_mut().find(|t| t.name() == "pdf_tool");
        let tool = match tool {
            Some(t) => t,
            None => {
                return Err(KowalskiError::ToolExecution(
                    "pdf_tool not found".to_string(),
                ));
            }
        };
        let input = ToolInput::new(
            "parse".to_string(),
            content.to_string(),
            serde_json::json!({}),
        );
        let output = tool.execute(input).await?;
        Ok(output.result["content"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }
}

#[async_trait]
impl Agent for AcademicAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        AcademicAgent::new(config).await
    }

    fn start_conversation(&mut self, model: &str) -> String {
        self.agent.base_mut().start_conversation(model)
    }

    fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.agent.base().get_conversation(id)
    }

    fn list_conversations(&self) -> Vec<&Conversation> {
        self.agent.base().list_conversations()
    }

    fn delete_conversation(&mut self, id: &str) -> bool {
        self.agent.base_mut().delete_conversation(id)
    }

    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<reqwest::Response, KowalskiError> {
        self.agent
            .base_mut()
            .chat_with_history(conversation_id, content, role)
            .await
    }

    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<kowalski_core::conversation::Message>, KowalskiError> {
        self.agent
            .base_mut()
            .process_stream_response(conversation_id, chunk)
            .await
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        self.agent
            .base_mut()
            .add_message(conversation_id, role, content)
            .await;
    }

    fn name(&self) -> &str {
        self.agent.base().name()
    }

    fn description(&self) -> &str {
        self.agent.base().description()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kowalski_core::config::Config;

    #[tokio::test]
    async fn test_academic_agent_creation() {
        let config = Config::default();
        let agent = AcademicAgent::new(config).await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_academic_agent_conversation() {
        let config = Config::default();
        let mut agent = AcademicAgent::new(config).await.unwrap();
        let conv_id = agent.start_conversation("test-model");
        assert!(!conv_id.is_empty());
    }
}
