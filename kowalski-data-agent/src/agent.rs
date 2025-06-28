use crate::config::DataAgentConfig;
use async_trait::async_trait;
use kowalski_agent_template::templates::general::GeneralTemplate;
use kowalski_agent_template::TemplateAgent;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::Conversation;
use kowalski_core::error::KowalskiError;
use kowalski_core::role::Role;
use kowalski_core::tools::{Tool, ToolInput};
use kowalski_tools::data::CsvTool;
use serde::{Deserialize, Serialize};

/// DataAgent: A specialized agent for data analysis and processing tasks
/// This agent is built on top of the TemplateAgent and provides data-specific functionality
pub struct DataAgent {
    agent: TemplateAgent,
    config: DataAgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvAnalysisResult {
    pub headers: Vec<String>,
    pub total_rows: usize,
    pub total_columns: usize,
    pub summary: serde_json::Value,
}

impl DataAgent {
    /// Creates a new DataAgent with the specified configuration
    pub async fn new(_config: Config) -> Result<Self, KowalskiError> {
        // TODO: Convert Config to DataAgentConfig if needed
        let data_config = DataAgentConfig::default();
        let csv_tool = CsvTool::new(data_config.max_rows, data_config.max_columns);

        let tools: Vec<Box<dyn Tool + Send + Sync>> = vec![Box::new(csv_tool)];
        let builder = GeneralTemplate::create_agent(
            tools,
            Some("You are a data analysis assistant specialized in processing and analyzing structured data.".to_string()),
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

    /// Processes a CSV file
    pub async fn process_csv(&self, csv_content: &str) -> Result<CsvAnalysisResult, KowalskiError> {
        let mut tools = self.agent.tool_chain.write().await;
        let tool = tools.iter_mut().find(|t| t.name() == "csv_tool");
        let tool = match tool {
            Some(t) => t,
            None => {
                return Err(KowalskiError::ToolExecution(
                    "csv_tool not found".to_string(),
                ));
            }
        };
        let input = ToolInput::new(
            "process_csv".to_string(),
            csv_content.to_string(),
            serde_json::json!({
                "max_rows": self.config.max_rows,
                "max_columns": self.config.max_columns
            }),
        );
        let output = tool.execute(input).await?;
        
        let result = output.result;
        Ok(CsvAnalysisResult {
            headers: result["headers"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            total_rows: result["total_rows"].as_u64().unwrap_or_default() as usize,
            total_columns: result["total_columns"].as_u64().unwrap_or_default() as usize,
            summary: result["summary"].clone(),
        })
    }

    /// Analyzes data statistics
    pub async fn analyze_data(&self, csv_content: &str) -> Result<serde_json::Value, KowalskiError> {
        let mut tools = self.agent.tool_chain.write().await;
        let tool = tools.iter_mut().find(|t| t.name() == "csv_tool");
        let tool = match tool {
            Some(t) => t,
            None => {
                return Err(KowalskiError::ToolExecution(
                    "csv_tool not found".to_string(),
                ));
            }
        };
        let input = ToolInput::new(
            "analyze_csv".to_string(),
            csv_content.to_string(),
            serde_json::json!({}),
        );
        let output = tool.execute(input).await?;
        Ok(output.result)
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
