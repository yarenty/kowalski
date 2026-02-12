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
    // tool_chain removed in favor of base.tool_manager
    pub task_handlers: Arc<RwLock<HashMap<String, Box<dyn TaskHandler>>>>,
}

#[async_trait]
pub trait TaskHandler: Send + Sync {
    async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError>;
}

impl TemplateAgent {
    /// Creates a new TemplateAgent with the specified configuration
    pub async fn new(config: Config) -> Result<Self, KowalskiError> {
        use crate::memory::helpers::create_memory_providers;
        use crate::llm::create_llm_provider;
        
        // Create LLM provider
        let llm_provider = create_llm_provider(&config)?;

        let (working_memory, episodic_memory, semantic_memory) = 
            create_memory_providers(&config).await?;
        
        let base = BaseAgent::new(
            config.clone(),
            "Template Agent",
            "A base implementation for building specialized agents",
            llm_provider,
            working_memory,
            episodic_memory,
            semantic_memory,
            crate::tools::manager::ToolManager::new(),
        )
        .await?;
        let template_config = TemplateAgentConfig::from(config);
        let task_handlers = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            base,
            config: template_config,
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
    /// Registers a tool with the agent
    pub async fn register_tool(
        &self,
        tool: Box<dyn Tool + Send + Sync>,
    ) -> Result<(), KowalskiError> {
        self.base.tool_manager.register_boxed(tool);
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
        // Delegate to base agent which uses ToolManager
        // But here we might want to convert Value to ToolInput?
        // BaseAgent::execute_tool already does that handling (lines 508+ in agent/mod.rs)
        // Wait, `execute_tool` in TemplateAgent was likely calling internal `tool.execute`.
        // Now we call `self.base.execute_tool(...)`.
        // `BaseAgent` implements specific `execute_tool`.
        // Does `BaseAgent` struct have `execute_tool` method?
        // `impl Agent for BaseAgent` has it.
        // `TemplateAgent` holds `base: BaseAgent`.
        // We can call `self.base.tool_manager.execute(...)`.
        // But we need to construct `ToolInput`.
        
        // Replicating BaseAgent logic:
        let task_type = tool_input
            .get("task")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let content = tool_input
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let input = crate::tools::ToolInput::new(task_type, content, tool_input.clone());
        self.base.tool_manager.execute(tool_name, input).await
    }

    /// Executes a task using the appropriate tool or handler
    /// Executes a task using the appropriate tool or handler
    pub async fn execute_task(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        // First try to find a matching tool via ToolManager
        // ToolManager::execute requires tool name. 
        // Here we don't have tool name, just TaskType?
        // `ToolInput` has `task_type`.
        // Old logic iterated ALL tools and tried to execute.
        // `ToolManager` is Map-based (by tool name).
        // It doesn't support "try all tools".
        // 
        // We need to deprecate this "try everything" approach or implement a way to map task->tool.
        // `ToolChain` supported this.
        // `ToolManager` should support task handler mapping?
        //
        // If we want to keep `execute_task` working, we need `ToolManager` to support it.
        // For now, let's say "Not supported" or iterate if we must.
        // `ToolManager::list_tools` gives names. Then we can try execute?
        // But `execute` requires name.
        // 
        // If `DataAgent` relies on this...
        // `DataAgent` doesn't call `execute_task` explicitly in what I saw.
        // The Prompt says "Use tool...".
        // `chat_with_tools` calls `execute_tool` (by name).
        //
        // So `execute_task` might be legacy `TaskHandler` stuff.
        // I will comment it out or return error for now to encourage explicit tool use.
        // Or iterate `tool_manager.list_tools()` keys.
        
        let tools = self.base.tool_manager.list_tools().await;
        for (name, _) in tools {
            // This is inefficient but mimics old behavior.
            // But we need to call `execute` which takes `ToolInput`.
            // `execute` finds tool by name.
            if let Ok(output) = self.base.tool_manager.execute(&name, input.clone()).await {
                 return Ok(output);
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
        self.base.tool_manager.list_tools().await
    }
}
