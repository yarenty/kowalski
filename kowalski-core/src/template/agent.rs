use crate::agent::BaseAgent;
use crate::config::Config;
use crate::error::KowalskiError;
use crate::template::config::TemplateAgentConfig;
use crate::tools::{TaskType, Tool, ToolInput, ToolOutput};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// TemplateAgent: A base agent implementation that provides common functionality
/// for specialized agents to build upon
pub struct TemplateAgent {
    base: BaseAgent,
    config: TemplateAgentConfig,
    pub tool_chain: Arc<RwLock<Vec<Box<dyn Tool + Send + Sync>>>>,
    pub task_handlers: Arc<RwLock<HashMap<String, Box<dyn TaskHandler>>>>,
}

#[async_trait]
pub trait TaskHandler: Send + Sync {
    async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError>;
}

impl TemplateAgent {
    /// Creates a new TemplateAgent with the specified configuration
    pub async fn new(config: Config) -> Result<Self, KowalskiError> {
        let base = BaseAgent::new(
            config.clone(),
            "Template Agent",
            "A base implementation for building specialized agents",
        )
        .await?;
        let template_config = TemplateAgentConfig::from(config);
        let tool_chain = Arc::new(RwLock::new(Vec::new()));
        let task_handlers = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            base,
            config: template_config,
            tool_chain,
            task_handlers,
        })
    }

    /// Configures the system prompt for the agent
    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.config.system_prompt = prompt.to_string();
        self
    }

    /// Gets the underlying base agent
    pub fn base(&self) -> &BaseAgent {
        &self.base
    }

    /// Gets a mutable reference to the underlying base agent
    pub fn base_mut(&mut self) -> &mut BaseAgent {
        &mut self.base
    }

    /// Gets the template configuration
    pub fn config(&self) -> &TemplateAgentConfig {
        &self.config
    }

    /// Gets a mutable reference to the template configuration
    pub fn config_mut(&mut self) -> &mut TemplateAgentConfig {
        &mut self.config
    }

    /// Registers a tool with the agent
    pub async fn register_tool(
        &self,
        tool: Box<dyn Tool + Send + Sync>,
    ) -> Result<(), KowalskiError> {
        let mut tools = self.tool_chain.write().await;
        tools.push(tool);
        Ok(())
    }

    /// Registers a task handler with the agent
    pub async fn register_task_handler(
        &self,
        task_type: impl TaskType,
        handler: Box<dyn TaskHandler>,
    ) {
        let mut handlers = self.task_handlers.write().await;
        handlers.insert(task_type.name().to_string(), handler);
    }

    pub async fn execute_tool(
        &mut self,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) -> Result<ToolOutput, KowalskiError> {
        let mut tools = self.tool_chain.write().await;
        if let Some(tool) = tools.iter_mut().find(|t| t.name() == tool_name) {
            let tool_input_struct = ToolInput {
                task_type: tool_name.to_string(),
                content: "".to_string(),
                parameters: tool_input.clone(),
            };
            tool.execute(tool_input_struct).await
        } else {
            Err(KowalskiError::ToolExecution(format!(
                "Tool '{}' not found",
                tool_name
            )))
        }
    }

    /// Executes a task using the appropriate tool or handler
    pub async fn execute_task(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        // First try to find a matching tool
        let mut tools = self.tool_chain.write().await;
        for tool in tools.iter_mut() {
            // Try to execute the tool and check if it succeeds
            match tool.execute(input.clone()).await {
                Ok(output) => return Ok(output),
                Err(_) => continue,
            }
        }

        // If no tool matches, try to find a task handler
        let handlers = self.task_handlers.read().await;
        if let Some(handler) = handlers.get(&input.task_type) {
            return handler.handle(input).await;
        }

        Err(KowalskiError::ToolExecution(format!(
            "No handler found for task type: {}",
            input.task_type
        )))
    }

    /// Lists all registered tools (name, description)
    pub async fn list_tools(&self) -> Vec<(String, String)> {
        let tools = self.tool_chain.read().await;
        tools
            .iter()
            .map(|t| (t.name().to_string(), t.description().to_string()))
            .collect()
    }
}
