use crate::error::KowalskiError;
use crate::tools::{TaskType, Tool, ToolInput, ToolOutput};
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for task handler functions
type TaskHandlerFn = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// A chain of tools that can be executed in sequence.
///
/// Prefer [`crate::tools::manager::ToolManager`] for new code (registration, JSON schema, MCP integration).
pub struct ToolChain {
    /// The tools in the chain
    tools: Vec<Box<dyn Tool + Send + Sync>>,
    /// Task type handlers
    task_handlers: HashMap<String, TaskHandlerFn>,
}

impl Default for ToolChain {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolChain {
    /// Create a new tool chain
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            task_handlers: HashMap::new(),
        }
    }

    /// Register a tool in the chain
    pub fn register_tool(&mut self, tool: Box<dyn Tool + Send + Sync>) {
        self.tools.push(tool);
    }

    /// Register a handler for a specific task type
    pub fn register_task_handler<T: TaskType, F>(&mut self, task_type: T, handler: F)
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.task_handlers
            .insert(task_type.name().to_string(), Arc::new(handler));
    }

    /// Execute the tool chain with the given input
    pub async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        // Check if we have a handler for this task type
        if let Some(handler) = self.task_handlers.get(&input.task_type) {
            if handler(&input.content) {
                // Find the first tool that can handle this task
                for tool in &mut self.tools {
                    match tool.execute(input.clone()).await {
                        Ok(output) => return Ok(output),
                        Err(_) => continue,
                    }
                }
                return Err(KowalskiError::ToolExecution(
                    "No tool could handle the task".to_string(),
                ));
            }
        }
        Err(KowalskiError::ToolExecution(
            "No handler found for task type".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::KowalskiError;
    use crate::tools::manager::ToolManager;
    use crate::tools::{Tool, ToolInput, ToolOutput, ToolParameter};
    use serde_json::json;

    struct MockTool;
    #[async_trait::async_trait]
    impl Tool for MockTool {
        async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                json!({ "result": input.content }),
                Some(json!({ "tool": "mock" })),
            ))
        }
        fn name(&self) -> &str {
            "mock_tool"
        }
        fn description(&self) -> &str {
            "A mock tool for testing."
        }
        fn parameters(&self) -> Vec<ToolParameter> {
            vec![]
        }
    }

    #[tokio::test]
    async fn tool_manager_executes_registered_tool() {
        let mgr = ToolManager::new();
        mgr.register(MockTool);
        let input = ToolInput::new(
            "mock_task".to_string(),
            "test content".to_string(),
            json!({}),
        );
        let result = mgr.execute("mock_tool", input).await;
        assert!(result.is_ok());
    }
}
