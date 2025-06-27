use crate::config::DataAgentConfig;
use crate::tools::{CsvTool, DataTaskType};
use kowalski_agent_template::builder::AgentBuilder;
use kowalski_agent_template::templates::general::GeneralTemplate;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::error::KowalskiError;
use serde_json::json;
use kowalski_core::conversation::{Conversation, Message};
use kowalski_core::role::Role;
use async_trait::async_trait;

/// DataAgent: A specialized agent for data analysis and processing tasks
/// This agent is built on top of the TemplateAgent and provides data-specific functionality
pub struct DataAgent {
    template: AgentBuilder,
    config: DataAgentConfig,
}

impl DataAgent {
    /// Creates a new DataAgent with the specified configuration
    pub async fn new(config: Config) -> Result<Self, KowalskiError> {
        let data_config = DataAgentConfig::from(config);
        let csv_tool = CsvTool::new(data_config.max_rows, data_config.max_columns);

        let builder = GeneralTemplate::create_agent(
            vec![Box::new(csv_tool) as Box<dyn Tool + Send + Sync>],
            Some("You are a data analysis assistant specialized in processing and analyzing structured data.".to_string()),
            Some(0.7),
        )
        .await
        .map_err(|e| KowalskiError::Config(e.to_string()))?;

        Ok(Self {
            template: builder,
            config: data_config,
        })
    }

    /// Processes a CSV file
    pub async fn process_csv(&self, file_path: &str) -> Result<String, KowalskiError> {
        let tool_input = ToolInput::new(
            DataTaskType::ProcessCsv.to_string(),
            file_path.to_string(),
            json!({
                "max_rows": self.config.max_rows,
                "max_columns": self.config.max_columns
            }),
        );
        let tool_output = self.template.build().await?.execute_task(tool_input).await?;
        Ok(tool_output.result["processed_data"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Analyzes data statistics
    pub async fn analyze_data(&self, data: serde_json::Value) -> Result<String, KowalskiError> {
        let tool_input = ToolInput::new(
            DataTaskType::AnalyzeData.to_string(),
            String::new(),
            data,
        );
        let tool_output = self.template.build().await?.execute_task(tool_input).await?;
        Ok(tool_output.result["analysis"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }
}

#[async_trait]
impl Agent for DataAgent {
    async fn execute_task(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        self.template.build().await?.execute_task(input).await
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        self.template
            .build()
            .await
            .expect("Failed to build agent")
            .add_message(conversation_id, role, content)
            .await;
    }

    async fn start_conversation(&self, model: &str) -> String {
        self.template
            .build()
            .await
            .expect("Failed to build agent")
            .start_conversation(model)
    }

    fn set_system_prompt(&mut self, prompt: &str) {
        self.template = self.template.with_system_prompt(prompt);
    }

    fn set_temperature(&mut self, temperature: f32) {
        self.template = self.template.with_temperature(temperature);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kowalski_core::error::KowalskiError;

    #[tokio::test]
    async fn test_data_agent_creation() {
        let config = Config::default();
        let agent = DataAgent::new(config).await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_data_agent_customization() {
        let config = Config::default();
        let mut agent = DataAgent::new(config).await.unwrap();
        
        agent.set_system_prompt("You are a specialized data analysis assistant.");
        agent.set_temperature(0.5);
    }
}
