use crate::tools::{TaskType, Tool, ToolInput, ToolOutput};
use std::collections::HashMap;
use std::sync::Arc;

/// A chain of tools that can be executed in sequence
pub struct ToolChain {
    /// The tools in the chain
    tools: Vec<Box<dyn Tool>>,
    /// Task type handlers
    task_handlers: HashMap<String, Arc<dyn Fn(&str) -> bool + Send + Sync>>,
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
    pub fn register_tool(&mut self, tool: Box<dyn Tool>) {
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
    pub async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
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
                return Err("No tool could handle the task".to_string());
            }
        }
        Err("No handler found for task type".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct MockTool;
    #[async_trait::async_trait]
    impl Tool for MockTool {
        async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
            Ok(ToolOutput::new(
                json!({ "result": input.content }),
                Some(json!({ "tool": "mock" })),
            ))
        }
    }

    #[derive(Debug, Clone)]
    struct MockTaskType;
    impl TaskType for MockTaskType {
        fn name(&self) -> &str {
            "mock_task"
        }

        fn description(&self) -> &str {
            "A mock task type for testing"
        }
    }

    impl std::fmt::Display for MockTaskType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "mock_task")
        }
    }

    #[tokio::test]
    async fn test_tool_chain() {
        let mut chain = ToolChain::new();
        chain.register_tool(Box::new(MockTool));
        chain.register_task_handler(MockTaskType, |_| true);

        let input = ToolInput::new(
            "mock_task".to_string(),
            "test content".to_string(),
            json!({}),
        );

        let result = chain.execute(input).await;
        assert!(result.is_ok());
    }
}
