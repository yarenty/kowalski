use crate::agent::BaseAgent;
use crate::config::Config;
use crate::error::KowalskiError;
use crate::mcp::McpHub;
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
        use crate::llm::create_llm_provider;
        use crate::memory::helpers::create_memory_providers;

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
        let mut template_config = TemplateAgentConfig::from(config.clone());
        let task_handlers = Arc::new(RwLock::new(HashMap::new()));

        if let Some(hub) = McpHub::new(&config.mcp.servers).await? {
            for proxy in hub.into_tool_proxies() {
                base.tool_manager.register_boxed(proxy);
            }
        }

        template_config.tool_prompt_appendix =
            Self::build_tool_prompt_appendix(&base.tool_manager).await;

        Ok(Self {
            base,
            config: template_config,
            task_handlers,
        })
    }

    async fn build_tool_prompt_appendix(
        tool_manager: &crate::tools::manager::ToolManager,
    ) -> String {
        let schema = tool_manager.generate_json_schema().await;
        let empty = schema.as_array().map(|a| a.is_empty()).unwrap_or(true);
        if empty {
            return String::new();
        }
        serde_json::to_string_pretty(&schema).map_or_else(
            |_| String::new(),
            |s| {
                format!(
                    "\n\n--- Available tools ---\nUse the agent's JSON tool-call format when invoking a tool.\n\n{s}"
                )
            },
        )
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

    /// Registers a tool and refreshes [`TemplateAgentConfig::tool_prompt_appendix`] from the
    /// current tool set so the next [`crate::agent::Agent::start_conversation`] includes updated schemas.
    pub async fn register_tool(
        &mut self,
        tool: Box<dyn Tool + Send + Sync>,
    ) -> Result<(), KowalskiError> {
        self.base.tool_manager.register_boxed(tool);
        self.config.tool_prompt_appendix =
            Self::build_tool_prompt_appendix(&self.base.tool_manager).await;
        Ok(())
    }

    /// Recomputes the tool schema appendix from the current [`BaseAgent::tool_manager`].
    /// Call this if tools are registered without going through [`Self::register_tool`].
    pub async fn refresh_tool_prompt_appendix(&mut self) {
        self.config.tool_prompt_appendix =
            Self::build_tool_prompt_appendix(&self.base.tool_manager).await;
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

    /// Prepare [`crate::llm::LLMProvider::chat_stream`] after the same context injection as chat (memories + user turn).
    pub async fn prepare_stream_turn(
        &mut self,
        conversation_id: &str,
        user: &str,
    ) -> Result<
        (
            String,
            Vec<crate::conversation::Message>,
            std::sync::Arc<dyn crate::llm::LLMProvider>,
        ),
        KowalskiError,
    > {
        self.base_mut()
            .prepare_stream_turn(conversation_id, user, None)
            .await
    }

    pub async fn prepare_stream_turn_with_options(
        &mut self,
        conversation_id: &str,
        user: &str,
        use_memory: bool,
    ) -> Result<
        (
            String,
            Vec<crate::conversation::Message>,
            std::sync::Arc<dyn crate::llm::LLMProvider>,
        ),
        KowalskiError,
    > {
        self.base_mut()
            .prepare_stream_turn_with_options(conversation_id, user, None, use_memory)
            .await
    }

    /// Like [`crate::agent::Agent::chat_with_tools`], but streams token deltas only for the LLM
    /// completion after at least one tool execution (see [`crate::agent::BaseAgent::chat_with_tools_stream_final`]).
    pub async fn chat_with_tools_stream_final(
        &mut self,
        conversation_id: &str,
        user_input: &str,
        token_tx: &tokio::sync::mpsc::Sender<String>,
    ) -> Result<String, KowalskiError> {
        self.base_mut()
            .chat_with_tools_stream_final(conversation_id, user_input, token_tx)
            .await
    }

    pub async fn chat_with_tools_with_options(
        &mut self,
        conversation_id: &str,
        user_input: &str,
        use_memory: bool,
    ) -> Result<String, KowalskiError> {
        self.base_mut()
            .chat_with_tools_with_options(conversation_id, user_input, use_memory)
            .await
    }

    pub async fn chat_with_tools_stream_final_with_options(
        &mut self,
        conversation_id: &str,
        user_input: &str,
        token_tx: &tokio::sync::mpsc::Sender<String>,
        use_memory: bool,
    ) -> Result<String, KowalskiError> {
        self.base_mut()
            .chat_with_tools_stream_final_with_options(
                conversation_id,
                user_input,
                token_tx,
                use_memory,
            )
            .await
    }

    pub async fn preview_memory_debug(
        &self,
        conversation_id: &str,
        user_input: &str,
        use_memory: bool,
    ) -> crate::agent::MemoryDebugInfo {
        self.base()
            .preview_memory_debug(conversation_id, user_input, use_memory)
            .await
    }

    pub fn replace_conversation_messages(
        &mut self,
        conversation_id: &str,
        messages: Vec<crate::conversation::Message>,
    ) -> Result<(), KowalskiError> {
        if let Some(conv) = self.base_mut().conversations.get_mut(conversation_id) {
            conv.messages = messages;
            Ok(())
        } else {
            Err(KowalskiError::ConversationNotFound(
                conversation_id.to_string(),
            ))
        }
    }
}

#[async_trait]
impl crate::agent::Agent for TemplateAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        TemplateAgent::new(config).await
    }

    fn start_conversation(&mut self, model: &str) -> String {
        let mut system_prompt = self
            .base
            .system_prompt
            .clone()
            .unwrap_or_else(|| self.config.system_prompt.clone());
        if system_prompt.trim().is_empty() {
            system_prompt = "You are a helpful assistant.".to_string();
        }
        system_prompt.push_str(&self.config.tool_prompt_appendix);
        let conv_id = self.base_mut().start_conversation(model);
        if let Some(conversation) = self.base_mut().conversations.get_mut(&conv_id) {
            conversation.add_message("system", &system_prompt);
        }
        conv_id
    }

    fn get_conversation(&self, id: &str) -> Option<&crate::conversation::Conversation> {
        self.base().get_conversation(id)
    }

    fn list_conversations(&self) -> Vec<&crate::conversation::Conversation> {
        self.base().list_conversations()
    }

    fn delete_conversation(&mut self, id: &str) -> bool {
        self.base_mut().delete_conversation(id)
    }

    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<crate::role::Role>,
    ) -> Result<String, KowalskiError> {
        self.base_mut()
            .chat_with_history(conversation_id, content, role)
            .await
    }

    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<crate::conversation::Message>, KowalskiError> {
        self.base_mut()
            .process_stream_response(conversation_id, chunk)
            .await
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        self.base_mut()
            .add_message(conversation_id, role, content)
            .await;
    }

    async fn execute_tool(
        &mut self,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) -> Result<ToolOutput, KowalskiError> {
        self.execute_tool(tool_name, tool_input).await
    }

    async fn list_tools(&self) -> Vec<(String, String)> {
        self.list_tools().await
    }

    fn export_conversation(&self, id: &str) -> Result<String, KowalskiError> {
        self.base().export_conversation(id)
    }

    fn import_conversation(&mut self, json_str: &str) -> Result<String, KowalskiError> {
        self.base_mut().import_conversation(json_str)
    }

    fn name(&self) -> &str {
        "Template Agent"
    }

    fn description(&self) -> &str {
        "A generic template agent wrapper."
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
