use crate::config::AcademicAgentConfig;
use async_trait::async_trait;
use kowalski_agent_template::TemplateAgent;
use kowalski_agent_template::default::DefaultTemplate;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::Conversation;
use kowalski_core::error::KowalskiError;
use kowalski_core::role::Role;
use kowalski_core::tools::{Tool, ToolOutput};
use kowalski_tools::document::PdfTool;
use kowalski_tools::fs::FsTool;
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
        let fs_tool = FsTool::new();
        let tools: Vec<Box<dyn Tool + Send + Sync>> =
            vec![Box::new(search_tool), Box::new(pdf_tool), Box::new(fs_tool)];

        let system_prompt = r#"You are an academic research assistant. You can search for papers, process PDF files, and interact with the filesystem.

AVAILABLE TOOLS:
1. web_search - Search the web for academic papers.
   - parameters: { "query": "search query" }
2. pdf_tool - Process a PDF file from a given path.
   - parameters: { "file_path": "/path/to/file.pdf", "extract_text": true, "extract_metadata": false, "extract_images": false }
3. fs_tool - Filesystem operations (list directories, find files, read file contents).
   - task: "list_dir" - List files and directories in a given path. Parameters: { "path": "/some/dir" }
   - task: "find_files" - Recursively find files matching a pattern. Parameters: { "dir": "/some/dir", "pattern": ".pdf" }
   - task: "get_file_contents" - Get the full contents of a file. Parameters: { "path": "/some/file.txt" }
   - task: "get_file_first_lines" - Get the first N lines of a file. Parameters: { "path": "/some/file.txt", "num_lines": 10 }
   - task: "get_file_last_lines" - Get the last N lines of a file. Parameters: { "path": "/some/file.txt", "num_lines": 10 }

TOOL USAGE INSTRUCTIONS:
- Use "web_search" to find papers on a topic.
- Use "pdf_tool" to analyze a paper you have the path to.
- Use "fs_tool" for filesystem operations.
- Specify the task and required parameters for each tool.

RESPONSE FORMAT:
When you need to use a tool, respond with JSON in this exact format:
{
  "name": "fs_tool",
  "parameters": {
    "task": "list_dir",
    "path": "/some/dir"
  },
  "reasoning": "I need to list the contents of this directory."
}

When you have a final answer, respond normally without JSON formatting."#
            .to_string();
        let system_prompt_clone = system_prompt.clone();
        let builder = DefaultTemplate::create_agent(tools, Some(system_prompt), Some(0.7))
            .await
            .map_err(|e| KowalskiError::Configuration(e.to_string()))?;
        let mut agent = builder.build().await?;
        // Ensure the system prompt is set on the base agent
        agent.base_mut().set_system_prompt(&system_prompt_clone);
        Ok(Self {
            agent,
            config: academic_config,
        })
    }

    pub async fn list_tools(&self) -> Vec<(String, String)> {
        self.agent.list_tools().await
    }
}

#[async_trait]
impl Agent for AcademicAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        AcademicAgent::new(config).await
    }

    fn start_conversation(&mut self, model: &str) -> String {
        let system_prompt = {
            let base = self.agent.base();
            base.system_prompt
                .as_deref()
                .unwrap_or("You are a helpful assistant.")
                .to_string()
        };
        let conv_id = self.agent.base_mut().start_conversation(model);
        if let Some(conversation) = self.agent.base_mut().conversations.get_mut(&conv_id) {
            conversation.add_message("system", &system_prompt);
        }
        conv_id
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
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
