use crate::config::DataAgentConfig;
use async_trait::async_trait;
use kowalski_agent_template::TemplateAgent;
use kowalski_agent_template::templates::general::GeneralTemplate;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::Conversation;
use kowalski_core::error::KowalskiError;
use kowalski_core::role::Role;
use kowalski_core::tools::{Tool, ToolOutput};
use kowalski_tools::csv::CsvTool;
use kowalski_tools::fs::FsTool;
use reqwest::Response;

/// DataAgent: A specialized agent for data analysis and processing tasks
/// This agent is built on top of the TemplateAgent and provides data-specific functionality
///
/// New: Can now analyze CSV files directly from a file path using the 'process_csv_path' tool task.
#[allow(dead_code)]
pub struct DataAgent {
    agent: TemplateAgent,
    config: DataAgentConfig,
}

impl DataAgent {
    /// Creates a new DataAgent with the specified configuration
    pub async fn new(_config: Config) -> Result<Self, KowalskiError> {
        // TODO: Convert Config to DataAgentConfig if needed
        let data_config = DataAgentConfig::default();
        let csv_tool = CsvTool::new(data_config.max_rows, data_config.max_columns);
        let fs_tool = FsTool::new();
        let tools: Vec<Box<dyn Tool + Send + Sync>> = vec![Box::new(csv_tool), Box::new(fs_tool)];

        let system_prompt = r#"You are a data analysis assistant.

AVAILABLE TOOLS:
csv_tool - For all CSV data analysis.
   - task: "process_csv" - Analyze CSV data. Parameters: { "content": "CSV file contents as a string" }
   - task: "process_csv_path" - Analyze CSV data from a file path. Parameters: { "path": "/some/file.csv" }

TOOL USAGE INSTRUCTIONS:
- ALWAYS use tools when asked about files, directories, or CSV data.
- NEVER give instructions, shell commands, or say you cannot access the filesystem.
- When asked to analyze a CSV file, you can now use either:
  1. fs_tool with task "get_file_contents" to read the file (provide the file path), then csv_tool with task "process_csv" and pass the file contents as the "content" parameter.
  2. OR, use csv_tool with task "process_csv_path" and provide the file path directly as the "path" parameter.
- Respond ONLY with JSON in this exact format for each tool call:

{
  "name": "csv_tool",
  "parameters": { "task": "process_csv_path", "path": "/opt/data/example.csv" },
  "reasoning": "User asked to analyze a CSV file from a path."
}

When you have a final answer, respond normally without JSON formatting. NEVER give instructions, shell commands, or say you cannot access the filesystem. ALWAYS use the tool call format for such requests.
"#
            .to_string();
        let system_prompt_clone = system_prompt.clone();
        println!(
            "[DEBUG] System prompt sent to LLM:\n{}",
            system_prompt_clone
        );

        let builder = GeneralTemplate::create_agent(tools, Some(system_prompt), Some(0.7))
            .await
            .map_err(|e| KowalskiError::Configuration(e.to_string()))?;
        let mut agent = builder.build().await?;
        // Ensure the system prompt is set on the base agent
        agent.base_mut().set_system_prompt(&system_prompt_clone);
        Ok(Self {
            agent,
            config: data_config,
        })
    }

    pub async fn list_tools(&self) -> Vec<(String, String)> {
        self.agent.list_tools().await
    }

    /// Analyze a CSV file from a file path using the csv_tool
    pub async fn process_csv_path(
        &mut self,
        path: &str,
    ) -> Result<serde_json::Value, KowalskiError> {
        let params = serde_json::json!({
            "task": "process_csv_path",
            "path": path
        });
        let output = self.execute_tool("csv_tool", &params).await?;
        Ok(output.result)
    }
}

#[async_trait]
impl Agent for DataAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        DataAgent::new(config).await
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
        "Data Agent"
    }

    fn description(&self) -> &str {
        "A specialized agent for data analysis tasks."
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
    async fn test_data_agent_creation() {
        let config = Config::default();
        let agent = DataAgent::new(config).await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_data_agent_conversation() {
        let config = Config::default();
        let mut agent = DataAgent::new(config).await.unwrap();
        let conv_id = agent.start_conversation("test-model");
        assert!(!conv_id.is_empty());
    }
}
