use crate::agent::TemplateAgent;
use crate::config::TemplateAgentConfig;
use kowalski_core::agent::BaseAgent;
use kowalski_core::config::Config;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::Tool;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::agent::TaskHandler;

pub struct AgentBuilder {
    base: BaseAgent,
    config: TemplateAgentConfig,
    tool_chain: Arc<RwLock<Vec<Box<dyn Tool + Send + Sync>>>>,
    task_handlers: Arc<RwLock<HashMap<String, Box<dyn TaskHandler>>>>,
    system_prompt: String,
    temperature: f32,
    tools: Vec<Box<dyn Tool + Send + Sync>>,
}

impl AgentBuilder {
    /// Creates a new AgentBuilder with default configuration
    pub async fn new() -> Self {
        let config = TemplateAgentConfig::default();
        let base = BaseAgent::new(
            Config::default(),
            "Template Agent",
            "A base implementation for building specialized agents",
        ).await.expect("Failed to create base agent");

        Self {
            base,
            config,
            tool_chain: Arc::new(RwLock::new(Vec::new())),
            task_handlers: Arc::new(RwLock::new(HashMap::new())),
            system_prompt: String::new(),
            temperature: 0.7,
            tools: Vec::new(),
        }
    }

    /// Sets the agent's system prompt
    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = prompt.to_string();
        self
    }

    /// Sets the temperature for responses
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Adds a tool to the agent
    pub fn with_tool<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    /// Adds multiple tools to the agent
    pub fn with_tools(mut self, tools: Vec<Box<dyn Tool + Send + Sync>>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Builds the final agent
    pub async fn build(self) -> Result<TemplateAgent, KowalskiError> {
        // Configure base agent
        // let mut base = self.base;
        // base.set_temperature(self.temperature);
        // if !self.system_prompt.is_empty() {
        //     base.set_system_prompt(&self.system_prompt);
        // }

        // Create template agent
        let mut agent = TemplateAgent::new(Config::default()).await?;

        // Register tools
        for tool in self.tools {
            agent.register_tool(tool).await?;
        }

        Ok(agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(Debug)]
    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock_tool"
        }

        fn description(&self) -> &str {
            "Mock tool for testing"
        }

        fn parameters(&self) -> Vec<ToolParameter> {
            Vec::new()
        }

        async fn execute(&mut self, _input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput {
                result: json!({ "status": "success" }),
                metadata: None,
            })
        }
    }

    #[tokio::test]
    async fn test_builder_with_tool() {
        let builder = AgentBuilder::new()
            .await.with_system_prompt("You are a helpful assistant")
            .with_tool(MockTool)
            .with_temperature(0.5);

        let agent = builder.build().await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_builder_with_multiple_tools() {
        let tools = vec![Box::new(MockTool) as Box<dyn Tool + Send + Sync>];
        let builder = AgentBuilder::new()
            .await.with_system_prompt("You are a helpful assistant")
            .with_tools(tools)
            .with_temperature(0.5);

        let agent = builder.build().await;
        assert!(agent.is_ok());
    }
}
