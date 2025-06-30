use crate::config::TemplateAgentConfig;
use async_trait::async_trait;
use kowalski_core::agent::BaseAgent;
use kowalski_core::config::Config;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{TaskType, Tool, ToolInput, ToolOutput};
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct MockTool {
        task_type: String,
    }

    #[async_trait]
    impl Tool for MockTool {
        async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            if input.task_type == self.task_type {
                Ok(ToolOutput::new(
                    json!({
                        "status": "success",
                        "message": "Mock tool executed successfully",
                        "input": input.content
                    }),
                    Some(json!({ "tool": "mock" })),
                ))
            } else {
                Err(KowalskiError::ToolExecution(
                    "Task type mismatch".to_string(),
                ))
            }
        }
        fn name(&self) -> &str {
            "mock_tool"
        }
        fn description(&self) -> &str {
            "Mock tool for testing"
        }
        fn parameters(&self) -> Vec<kowalski_core::tools::ToolParameter> {
            Vec::new()
        }
    }

    struct MockTaskHandler;

    #[async_trait]
    impl TaskHandler for MockTaskHandler {
        async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                json!({
                    "status": "success",
                    "message": "Mock handler executed successfully",
                    "input": input.content
                }),
                Some(json!({ "handler": "mock" })),
            ))
        }
    }

    #[derive(Debug, Clone, Copy)]
    enum MockTaskType {
        Test,
    }

    impl TaskType for MockTaskType {
        fn name(&self) -> &str {
            match self {
                MockTaskType::Test => "test",
            }
        }

        fn description(&self) -> &str {
            "A mock task type for testing"
        }
    }

    impl std::fmt::Display for MockTaskType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name())
        }
    }

    #[tokio::test]
    async fn test_template_agent() {
        let config = Config::default();
        let agent = TemplateAgent::new(config).await.unwrap();

        // Register mock tool
        let tool = Box::new(MockTool {
            task_type: "test".to_string(),
        });
        agent.register_tool(tool).await.unwrap();

        // Register task handler
        let handler = Box::new(MockTaskHandler);
        agent
            .register_task_handler(MockTaskType::Test, handler)
            .await;

        // Test task execution
        let input = ToolInput::new("test".to_string(), "test content".to_string(), json!({}));

        let result = agent.execute_task(input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.result["status"], "success");
        assert_eq!(output.metadata.unwrap()["tool"], "mock");
    }
}
