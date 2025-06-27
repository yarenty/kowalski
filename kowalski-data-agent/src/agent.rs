use crate::config::DataAgentConfig;
use crate::tools::CsvTool;
use async_trait::async_trait;
use kowalski_agent_template::TemplateAgent;
use kowalski_core::agent::BaseAgent;
use kowalski_core::config::Config;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DataAgent {
    template: TemplateAgent,
    config: DataAgentConfig,
}

impl DataAgent {
    pub fn new(config: Config) -> Result<Self, KowalskiError> {
        let template = TemplateAgent::new(config.clone())?;
        let data_config = DataAgentConfig::from(config);
        let agent = Self {
            template,
            config: data_config,
        };
        let csv_tool = Box::new(CsvTool {});
        agent.register_tool(csv_tool);
        Ok(agent)
    }

    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.config.system_prompt = prompt.to_string();
        self
    }

    pub fn base(&self) -> &BaseAgent {
        self.template.base()
    }

    pub fn base_mut(&mut self) -> &mut BaseAgent {
        self.template.base_mut()
    }

    pub async fn register_tool(&self, tool: Box<dyn Tool>) {
        self.template.register_tool(tool).await;
    }

    pub async fn execute_task(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        self.template.execute_task(input).await
    }
}
