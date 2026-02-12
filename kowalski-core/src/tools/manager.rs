use crate::error::KowalskiError;
use crate::tools::{Tool, ToolInput, ToolOutput};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

/// Manages a collection of tools and handles their execution
#[derive(Clone)]
pub struct ToolManager {
    tools: Arc<RwLock<HashMap<String, Arc<Mutex<dyn Tool>>>>>,
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
             // We can't easily move Box<dyn Tool> into Arc<Mutex<dyn Tool>> without reallocation/wrapping?
             // Actually Mutex::new takes specific type.
             // We can wrap Box<dyn Tool> in Mutex? No, Mutex<T>. 
             // We want Mutex<dyn Tool>.
             // Use Arc::new(Mutex::new(tool)) where tool is the Box?
             // Arc::new(Mutex::new(tool)) creates Arc<Mutex<Box<dyn Tool>>>. 
             // We want Arc<Mutex<dyn Tool>>.
             // Typically we can't coerce Mutex<Box<dyn Tool>> to Mutex<dyn Tool>.
             //
             // The `register` method taking `T: Tool` works because we create `Mutex<T>` which coerces to `Mutex<dyn Tool>` behind Arc?
             // `Arc<Mutex<T>>` -> `Arc<Mutex<dyn Tool>>`. Yes, if we cast properly.
             // 
             // Let's stick to `register<T>` for now.
             // If we need to register existing `Box<dyn Tool>`, we might need unsafe or specific handling.
             // Or update the map to hold `Arc<Mutex<Box<dyn Tool>>>`? No, duplicate indirection.
             //
             // For now, `register<T>` covers static cases. 
             // The legacy `ToolChain` used `Vec<Box<dyn Tool>>`.
             // We might need to iterate over them.
             
             // Let's implement `register_arc` if we already have one.
             // Or just `register_tool` which takes `Arc<Mutex<dyn Tool>>`.
             tools.insert(tool.name().to_string(), Arc::new(Mutex::new(tool)) as Arc<Mutex<dyn Tool>>); 
             // This cast works if `tool` is `Box<dyn Tool>`? 
             // `Mutex::new(box)` -> `Mutex<Box<dyn Tool>>`.
             // `Box<dyn Tool>` implements `Tool`? (Usually strictly distinct, but with impl Tool for Box<dyn Tool> maybe).
             //
             // I'll check if `impl Tool for Box<dyn Tool>` exists. If not, I should add it to `tools/mod.rs`.
        }
    }
    
    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<Mutex<dyn Tool>>> {
        if let Ok(tools) = self.tools.read() {
            tools.get(name).cloned()
        } else {
            None
        }
    }

    /// Execute a tool
    pub async fn execute(&self, name: &str, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        let tool = self.get(name).ok_or_else(|| {
            KowalskiError::ToolExecution(format!("Tool '{}' not found", name))
        })?;
        
        let mut tool_guard = tool.lock().await;
        tool_guard.execute(input).await
    }

    /// Generate tool descriptions for LLM system prompt
    /// Note: This is now async because it needs to acquire locks on tools.
    pub async fn generate_tool_descriptions(&self) -> String {
        let tools_snapshot: Vec<Arc<Mutex<dyn Tool>>> = if let Ok(tools) = self.tools.read() {
             tools.values().cloned().collect()
        } else {
            return "Error accessing tools".to_string();
        };

        let mut descriptions = String::new();
        for tool in tools_snapshot {
            let tool_guard = tool.lock().await;
            descriptions.push_str(&format!("{}: {}\n", tool_guard.name(), tool_guard.description()));
        }
        descriptions
    }

    /// List all registered tools (name, description)
    pub async fn list_tools(&self) -> Vec<(String, String)> {
        let tools_snapshot: Vec<Arc<Mutex<dyn Tool>>> = if let Ok(tools) = self.tools.read() {
             tools.values().cloned().collect()
        } else {
            return Vec::new();
        };

        let mut result = Vec::new();
        for tool in tools_snapshot {
            let tool_guard = tool.lock().await;
            result.push((tool_guard.name().to_string(), tool_guard.description().to_string()));
        }
        result
    }

    /// Generate a JSON schema for all registered tools (OpenAI-style function calling format)
    pub async fn generate_json_schema(&self) -> serde_json::Value {
        let tools_snapshot: Vec<Arc<Mutex<dyn Tool>>> = if let Ok(tools) = self.tools.read() {
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
                param_info.insert("type".to_string(), serde_json::json!(format!("{:?}", param.parameter_type).to_lowercase()));
                param_info.insert("description".to_string(), serde_json::json!(param.description));
                
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
