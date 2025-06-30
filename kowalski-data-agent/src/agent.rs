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
use kowalski_tools::data::CsvTool;
use reqwest::Response;

/// DataAgent: A specialized agent for data analysis and processing tasks
/// This agent is built on top of the TemplateAgent and provides data-specific functionality
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

        let tools: Vec<Box<dyn Tool + Send + Sync>> = vec![Box::new(csv_tool)];
        
        let system_prompt = r#"You are a data analysis assistant. You have access to a tool for processing CSV data.

AVAILABLE TOOLS:
1. csv_tool - Processes and analyzes CSV data.
   - task: "process_csv" - Reads a CSV file and returns headers, records, and a summary.
   - task: "analyze_csv" - Reads a CSV file and returns a statistical summary.

TOOL USAGE INSTRUCTIONS:
- When you need to analyze CSV data, use the "csv_tool".
- Specify the task you want to perform ("process_csv" or "analyze_csv").
- Provide the CSV data in the "content" parameter.

RESPONSE FORMAT:
When you need to use a tool, respond with JSON in this exact format:
{
  "name": "csv_tool",
  "parameters": {
    "task": "process_csv",
    "content": "header1,header2\nvalue1,value2"
  },
  "reasoning": "I need to process this CSV data."
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
            config: data_config,
        })
    }
}

#[async_trait]
impl Agent for DataAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        DataAgent::new(config).await
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
        "Data Agent"
    }

    fn description(&self) -> &str {
        "A specialized agent for data analysis tasks."
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
