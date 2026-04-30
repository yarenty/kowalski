use crate::error::KowalskiError;
use crate::tools::{Tool, ToolInput, ToolOutput};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

type SharedTool = Arc<Mutex<dyn Tool>>;
type ToolMap = HashMap<String, SharedTool>;

/// Manages a collection of tools and handles their execution
#[derive(Clone)]
pub struct ToolManager {
    tools: Arc<RwLock<ToolMap>>,
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolManager {
    /// Create a new ToolManager
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a tool
    pub fn register<T: Tool + 'static>(&self, tool: T) {
        if let Ok(mut tools) = self.tools.write() {
            tools.insert(tool.name().to_string(), Arc::new(Mutex::new(tool)));
        }
    }

    /// Register a boxed tool (useful for dynamic dispatch)
    pub fn register_boxed(&self, tool: Box<dyn Tool>) {
        if let Ok(mut tools) = self.tools.write() {
            tools.insert(tool.name().to_string(), Arc::new(Mutex::new(tool)));
        }
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<SharedTool> {
        if let Ok(tools) = self.tools.read() {
            tools.get(name).cloned()
        } else {
            None
        }
    }

    /// Execute a tool
    pub async fn execute(&self, name: &str, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        let tool = self
            .get(name)
            .ok_or_else(|| KowalskiError::ToolExecution(format!("Tool '{}' not found", name)))?;

        let mut tool_guard = tool.lock().await;
        tool_guard.execute(input).await
    }

    /// Generate tool descriptions for LLM system prompt
    /// Note: This is now async because it needs to acquire locks on tools.
    pub async fn generate_tool_descriptions(&self) -> String {
        let tools_snapshot: Vec<SharedTool> = if let Ok(tools) = self.tools.read() {
            tools.values().cloned().collect()
        } else {
            return "Error accessing tools".to_string();
        };

        let mut descriptions = String::new();
        for tool in tools_snapshot {
            let tool_guard = tool.lock().await;
            descriptions.push_str(&format!(
                "{}: {}\n",
                tool_guard.name(),
                tool_guard.description()
            ));
        }
        descriptions
    }

    /// List all registered tools (name, description)
    pub async fn list_tools(&self) -> Vec<(String, String)> {
        let tools_snapshot: Vec<SharedTool> = if let Ok(tools) = self.tools.read() {
            tools.values().cloned().collect()
        } else {
            return Vec::new();
        };

        let mut result = Vec::new();
        for tool in tools_snapshot {
            let tool_guard = tool.lock().await;
            result.push((
                tool_guard.name().to_string(),
                tool_guard.description().to_string(),
            ));
        }
        result
    }

    /// Generate a JSON schema for all registered tools (OpenAI-style function calling format)
    pub async fn generate_json_schema(&self) -> serde_json::Value {
        let tools_snapshot: Vec<SharedTool> = if let Ok(tools) = self.tools.read() {
            tools.values().cloned().collect()
        } else {
            return serde_json::json!([]);
        };

        let mut functions = Vec::new();
        for tool in tools_snapshot {
            let tool_guard = tool.lock().await;
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            for param in tool_guard.parameters() {
                let mut param_info = serde_json::Map::new();
                param_info.insert(
                    "type".to_string(),
                    serde_json::json!(format!("{:?}", param.parameter_type).to_lowercase()),
                );
                param_info.insert(
                    "description".to_string(),
                    serde_json::json!(param.description),
                );

                if let Some(default) = param.default_value {
                    param_info.insert("default".to_string(), serde_json::json!(default));
                }

                properties.insert(param.name.clone(), serde_json::Value::Object(param_info));
                if param.required {
                    required.push(param.name);
                }
            }

            functions.push(serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool_guard.name(),
                    "description": tool_guard.description(),
                    "parameters": {
                        "type": "object",
                        "properties": properties,
                        "required": required
                    }
                }
            }));
        }

        serde_json::Value::Array(functions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{ParameterType, ToolParameter};
    use async_trait::async_trait;

    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        async fn execute(&mut self, _input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                serde_json::json!({"status": "success"}),
                None,
            ))
        }

        fn name(&self) -> &str {
            "mock_tool"
        }
        fn description(&self) -> &str {
            "A mock tool for testing"
        }
        fn parameters(&self) -> Vec<ToolParameter> {
            vec![ToolParameter {
                name: "input".to_string(),
                description: "test input".to_string(),
                required: true,
                default_value: None,
                parameter_type: ParameterType::String,
            }]
        }
    }

    #[tokio::test]
    async fn test_tool_manager_registration() {
        let manager = ToolManager::new();
        manager.register(MockTool);

        let tools = manager.list_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].0, "mock_tool");
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let manager = ToolManager::new();
        manager.register(MockTool);

        let input = ToolInput::new(
            "mock_task".to_string(),
            "test content".to_string(),
            serde_json::json!({"input": "test"}),
        );

        let result = manager.execute("mock_tool", input).await.unwrap();
        assert_eq!(result.result["status"], "success");
    }

    #[tokio::test]
    async fn test_generate_tool_descriptions() {
        let manager = ToolManager::new();
        manager.register(MockTool);

        let descriptions = manager.generate_tool_descriptions().await;
        assert!(descriptions.contains("mock_tool: A mock tool for testing"));
    }

    #[tokio::test]
    async fn test_generate_json_schema() {
        let manager = ToolManager::new();
        manager.register(MockTool);

        let schema = manager.generate_json_schema().await;
        let schema_array = schema.as_array().unwrap();

        assert_eq!(schema_array.len(), 1);
        assert_eq!(schema_array[0]["function"]["name"], "mock_tool");
        assert_eq!(
            schema_array[0]["function"]["parameters"]["required"][0],
            "input"
        );
    }
}
