use crate::agent::BaseAgent;
use crate::config::Config;
use crate::error::KowalskiError;
use crate::template::agent::TaskHandler;
use crate::template::agent::TemplateAgent;
use crate::template::config::TemplateAgentConfig;
use crate::tools::Tool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[allow(dead_code)]
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
        )
        .await
        .expect("Failed to create base agent");

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
        let agent = TemplateAgent::new(Config::default()).await?;

        // Register tools
        for tool in self.tools {
            agent.register_tool(tool).await?;
        }

        Ok(agent)
    }
}
