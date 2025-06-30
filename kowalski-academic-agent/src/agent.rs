use crate::config::AcademicAgentConfig;
use async_trait::async_trait;
use kowalski_agent_template::TemplateAgent;
use kowalski_agent_template::templates::general::GeneralTemplate;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::Conversation;
use kowalski_core::error::KowalskiError;
use kowalski_core::role::Role;
use kowalski_core::tools::{Tool, ToolOutput};
use kowalski_tools::document::PdfTool;
use kowalski_tools::web::WebSearchTool;
use reqwest::Response;

/// AcademicAgent: A specialized agent for academic tasks
/// This agent is built on top of the TemplateAgent and provides academic-specific functionality
#[allow(dead_code)]
pub struct AcademicAgent {
    agent: TemplateAgent,
    config: AcademicAgentConfig,
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

        let system_prompt = r#"You are an academic research assistant. You can search for papers and process PDF files.

AVAILABLE TOOLS:
1. web_search - Search the web for academic papers.
   - parameters: { "query": "search query" }
2. pdf_tool - Process a PDF file from a given path.
   - parameters: { "file_path": "/path/to/file.pdf", "extract_text": true, "extract_metadata": false, "extract_images": false }

TOOL USAGE INSTRUCTIONS:
- Use "web_search" to find papers on a topic.
- Use "pdf_tool" to analyze a paper you have the path to.

RESPONSE FORMAT:
When you need to use a tool, respond with JSON in this exact format:
{
  "name": "web_search",
  "parameters": {
    "query": "machine learning in healthcare"
  },
  "reasoning": "I need to find papers on this topic."
}

or

{
  "name": "pdf_tool",
  "parameters": {
    "file_path": "/path/to/downloaded/paper.pdf",
    "extract_text": true,
    "extract_metadata": true
  },
  "reasoning": "I need to extract text and metadata from this PDF."
}

When you have a final answer, respond normally without JSON formatting."#
            .to_string();

        let builder = GeneralTemplate::create_agent(
            tools,
            Some(system_prompt),
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
    ) -> Result<Response, KowalskiError> {
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

    async fn execute_tool(
        &mut self,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) -> Result<ToolOutput, KowalskiError> {
        self.agent.execute_tool(tool_name, tool_input).await
    }

    fn name(&self) -> &str {
        "Academic Agent"
    }

    fn description(&self) -> &str {
        "A specialized agent for academic research tasks."
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
