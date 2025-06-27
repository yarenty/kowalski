use kowalski_core::tools::{Tool, ToolInput, ToolOutput};
use kowalski_core::error::KowalskiError;
use tokio::runtime::Handle;

pub struct ToolManager {
    tools: Vec<Box<dyn Tool + Send + Sync>>,
}

impl Default for ToolManager {
         fn default() -> Self {
             Self::new()
         }
 }


impl ToolManager {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn register_tool<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.push(Box::new(tool));
    }

    pub fn with_tool_mut<F, R>(&mut self, name: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut (dyn Tool + Send + Sync)) -> R,
    {
        self.tools
            .iter_mut()
            .find(|t| t.name() == name)
            .map(|t| f(t.as_mut()))
    }

    pub async fn execute_tool(
        &mut self,
        name: &str,
        input: ToolInput,
    ) -> Result<ToolOutput, KowalskiError> {
        if let Some(result) = self.with_tool_mut(name, |tool| {
            tool.validate_input(&input)?;
            Handle::current().block_on(tool.execute(input))
        }) {
            result
        } else {
            Err(KowalskiError::ToolInvalidInput(format!("Tool not found: {}", name)))
        }
    }

    pub fn list_tools(&self) -> Vec<(String, String)> {
        self.tools
            .iter()
            .map(|t| (t.name().to_string(), t.description().to_string()))
            .collect()
    }
}
