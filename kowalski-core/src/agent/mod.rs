use crate::agent::types::StreamResponse;
use crate::config::Config;
use crate::conversation::Conversation;
use crate::conversation::Message;
use crate::error::KowalskiError;
use crate::memory::MemoryProvider;
use crate::memory::MemoryUnit;
use crate::memory::working::WorkingMemory;
use crate::role::Role;
use crate::tools::{ToolCall, ToolOutput};
use async_trait::async_trait;
use futures::StreamExt;
use log::debug;
use log::info;
use log::warn;
use serde_json;
use serde_json::json;
use std::any::Any;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod repl_trace;
pub mod types;

/// The core agent trait that all our specialized agents must implement.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Creates a new agent with the specified configuration.
    async fn new(config: Config) -> Result<Self, KowalskiError>
    where
        Self: Sized;

    /// Starts a new conversation
    fn start_conversation(&mut self, model: &str) -> String;

    /// Gets a conversation by ID
    fn get_conversation(&self, id: &str) -> Option<&Conversation>;

    /// Lists all conversations
    fn list_conversations(&self) -> Vec<&Conversation>;

    /// Deletes a conversation
    fn delete_conversation(&mut self, id: &str) -> bool;

    /// Chats with history (model messages) for the given conversation.
    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<String, KowalskiError>;

    /// Processes a stream response
    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<Message>, KowalskiError>;

    /// Adds a message to a conversation
    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str);

    /// Exports a conversation to a JSON string
    fn export_conversation(&self, id: &str) -> Result<String, KowalskiError>;

    /// Imports a conversation from a JSON string, returns the new conversation ID
    fn import_conversation(&mut self, json: &str) -> Result<String, KowalskiError>;

    /// Executes a tool with the given name and input.
    async fn execute_tool(
        &mut self,
        _tool_name: &str,
        _tool_input: &serde_json::Value,
    ) -> Result<ToolOutput, KowalskiError> {
        Err(KowalskiError::ToolExecution(
            "Tool execution not implemented for this agent".to_string(),
        ))
    }

    /// Chat with the agent using ReAct-style tool calling
    async fn chat_with_tools(
        &mut self,
        conversation_id: &str,
        user_input: &str,
    ) -> Result<String, KowalskiError> {
        let mut final_response = String::new();
        let mut current_input = user_input.to_string();
        let mut iteration_count = 0;
        const MAX_ITERATIONS: usize = 5; // Prevent infinite loops
        let mut last_tool_call: Option<(String, serde_json::Value)> = None;
        let mut tool_parse_hint_sent = false;

        debug!("Starting chat_with_tools for input: '{}'", user_input);

        while iteration_count < MAX_ITERATIONS {
            iteration_count += 1;
            debug!(" === ITERATION {} ===", iteration_count);
            debug!("Current input: '{}'", current_input);

            // Get response from LLM
            debug!("Calling LLM...");
            let response_text = self
                .chat_with_history(conversation_id, &current_input, None)
                .await?;

            // Print response (simulate streaming effect or just print)
            if repl_trace::repl_trace_enabled() {
                println!("[agent] {}", response_text);
            } else {
                println!("{}", response_text);
            }
            io::stdout()
                .flush()
                .map_err(|e| KowalskiError::Server(e.to_string()))?;

            let buffer = response_text.clone();
            debug!("Full LLM response: '{}'", buffer);

            // Try to extract JSON from mixed text response using robust utility
            debug!("Attempting to extract tool calls from response...");
            let tool_calls = crate::utils::json::extract_tool_calls(&buffer);

            if !tool_calls.is_empty() {
                // For now, we only process the first tool call found in one turn
                let tool_call = &tool_calls[0];

                // Detect repeated tool calls
                let tool_call_key = (tool_call.name.clone(), tool_call.parameters.clone());
                if let Some(last) = &last_tool_call {
                    if *last == tool_call_key {
                        debug!(
                            "Detected repeated tool call. Breaking loop to prevent infinite tool call loop."
                        );
                        break;
                    }
                }
                last_tool_call = Some(tool_call_key.clone());

                debug!("✅ Tool call successfully parsed!");
                debug!("Tool: {}", tool_call.name);
                debug!("Parameters: {}", tool_call.parameters);
                debug!("Reasoning: {:?}", tool_call.reasoning);

                if repl_trace::repl_trace_enabled() {
                    let params = serde_json::to_string(&tool_call.parameters)
                        .unwrap_or_else(|_| "{}".to_string());
                    println!("[tool] {} {}", tool_call.name, params);
                }

                let tool_result = match self
                    .execute_tool(&tool_call.name, &tool_call.parameters)
                    .await
                {
                    Ok(output) => output.result.to_string(),
                    Err(e) => {
                        let err_msg = format!("{}", e);
                        debug!("Tool execution failed: {}", err_msg);

                        // Basic fallback/chaining logic can be integrated here if needed
                        err_msg
                    }
                };

                let tool_message = format!("Tool result for {}: {}", tool_call.name, tool_result);
                self.add_message(conversation_id, "assistant", &tool_message)
                    .await;
                debug!("Added tool result to conversation");

                current_input = format!("Based on the tool result: {}", tool_result);
                debug!("Continuing with new input: '{}'", current_input);
                continue;
            }

            if crate::utils::json::looks_like_tool_json_attempt(&buffer) && !tool_parse_hint_sent {
                tool_parse_hint_sent = true;
                let preview: String = buffer.chars().take(400).collect();
                let total_chars = buffer.chars().count();
                warn!(
                    "Tool call JSON parse failed ({} chars); raw preview: {:?}",
                    total_chars, preview
                );
                self.add_message(conversation_id, "assistant", &buffer)
                    .await;
                const HINT: &str = "Your previous reply appeared to include a tool call but it could not be parsed as JSON. Reply with a single JSON object only: {\"name\": \"<tool_name>\", \"parameters\": { ... } } matching the available tools. No markdown fences or extra text.";
                current_input = HINT.to_string();
                debug!("Tool JSON parse failed; requesting one self-correction turn");
                continue;
            }

            // Not a tool call, this is the final answer
            final_response = buffer;
            self.add_message(conversation_id, "assistant", &final_response)
                .await;
            debug!("✅ Final response set: '{}'", final_response);

            if let Some(tool_call) = rule_based_tool_call(user_input) {
                debug!("Rule-based tool call triggered: {:?}", tool_call);
                let tool_result = self
                    .execute_tool(&tool_call.name, &tool_call.parameters)
                    .await;
                let tool_result_str = match tool_result {
                    Ok(output) => output.result.to_string(),
                    Err(e) => format!("Tool execution failed: {}", e),
                };
                self.add_message(conversation_id, "assistant", &tool_result_str)
                    .await;
                debug!("Rule-based tool result: {}", tool_result_str);
                return Ok(tool_result_str);
            }

            break;
        }

        if iteration_count >= MAX_ITERATIONS {
            warn!("Reached maximum iterations, returning current response");
        }

        debug!(
            "chat_with_tools completed after {} iterations",
            iteration_count
        );
        Ok(final_response)
    }

    /// Lists tools available to this agent
    async fn list_tools(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    fn name(&self) -> &str;

    /// Gets the agent's description
    fn description(&self) -> &str;

    fn as_any(&self) -> &dyn Any;
}

/// The base agent implementation that provides common functionality.
pub struct BaseAgent {
    pub client: reqwest::Client,
    pub config: Config,
    pub conversations: HashMap<String, Conversation>,
    pub name: String,
    pub description: String,
    pub system_prompt: Option<String>,
    // LLM Provider
    pub llm_provider: std::sync::Arc<dyn crate::llm::LLMProvider>,
    // Memory Tiers - now using dependency injection
    pub working_memory: std::sync::Arc<tokio::sync::Mutex<dyn MemoryProvider + Send + Sync>>,
    pub episodic_memory: std::sync::Arc<tokio::sync::Mutex<dyn MemoryProvider + Send + Sync>>,
    pub semantic_memory: std::sync::Arc<tokio::sync::Mutex<dyn MemoryProvider + Send + Sync>>,
    // Tool Manager
    pub tool_manager: crate::tools::manager::ToolManager,
}

impl BaseAgent {
    fn recent_conversation_context(messages: &[Message], max_items: usize) -> String {
        let mut recent: Vec<String> = messages
            .iter()
            .rev()
            .filter(|m| m.role != "system")
            .take(max_items)
            .map(|m| format!("[{}] {}", m.role, m.content))
            .collect();
        recent.reverse();
        recent.join("\n---\n")
    }

    async fn build_memory_context(&self, content: &str, use_memory: bool) -> String {
        if !use_memory {
            return String::new();
        }

        let working_memories = self
            .working_memory
            .lock()
            .await
            .retrieve(content, self.config.working_memory_retrieval_limit)
            .await
            .unwrap_or_default();

        let episodic_memories = self
            .episodic_memory
            .lock()
            .await
            .retrieve(content, self.config.episodic_memory_retrieval_limit)
            .await
            .unwrap_or_default();

        let semantic_memories = self
            .semantic_memory
            .lock()
            .await
            .retrieve(content, self.config.semantic_memory_retrieval_limit)
            .await
            .unwrap_or_default();

        let mut seen_ids = HashSet::new();
        let mut all_memories = Vec::new();
        for m in working_memories
            .into_iter()
            .chain(episodic_memories)
            .chain(semantic_memories)
        {
            if seen_ids.insert(m.id.clone()) {
                all_memories.push(m);
            }
        }

        if all_memories.is_empty() {
            return String::new();
        }

        let concatenated_memories = all_memories
            .iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<&str>>()
            .join("\n---\n");
        format!(
            "\n--- Relevant Memories ---\n{}\n--- End Memories ---",
            concatenated_memories
        )
    }

    pub async fn new(
        config: Config,
        name: &str,
        description: &str,
        llm_provider: std::sync::Arc<dyn crate::llm::LLMProvider>,
        working_memory: std::sync::Arc<tokio::sync::Mutex<dyn MemoryProvider + Send + Sync>>,
        episodic_memory: std::sync::Arc<tokio::sync::Mutex<dyn MemoryProvider + Send + Sync>>,
        semantic_memory: std::sync::Arc<tokio::sync::Mutex<dyn MemoryProvider + Send + Sync>>,
        tool_manager: crate::tools::manager::ToolManager,
    ) -> Result<Self, KowalskiError> {
        let client = reqwest::ClientBuilder::new()
            .http1_only()
            .pool_max_idle_per_host(0)
            .build()
            .map_err(KowalskiError::Request)?;

        info!("BaseAgent created with name: {}", name);

        Ok(Self {
            client,
            config,
            conversations: HashMap::new(),
            name: name.to_string(),
            description: description.to_string(),
            system_prompt: None,
            llm_provider,
            working_memory,
            episodic_memory,
            semantic_memory,
            tool_manager,
        })
    }

    pub fn set_temperature(&mut self, temperature: f32) {
        self.config.chat.temperature = temperature;
    }

    pub fn set_system_prompt(&mut self, prompt: &str) {
        self.system_prompt = Some(prompt.to_string());
    }

    /// Same memory + user turn as [`Agent::chat_with_history`], but returns owned messages for
    /// [`crate::llm::LLMProvider::chat_stream`] without calling the LLM (caller streams, then
    /// should [`Self::add_message`] with role `assistant` for the full reply).
    pub async fn prepare_stream_turn(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<(String, Vec<Message>, std::sync::Arc<dyn crate::llm::LLMProvider>), KowalskiError>
    {
        self.prepare_stream_turn_with_options(conversation_id, content, role, true)
            .await
    }

    pub async fn prepare_stream_turn_with_options(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
        use_memory: bool,
    ) -> Result<(String, Vec<Message>, std::sync::Arc<dyn crate::llm::LLMProvider>), KowalskiError>
    {
        let memory_context = self.build_memory_context(content, use_memory).await;

        let conversation = self
            .conversations
            .get_mut(conversation_id)
            .ok_or_else(|| KowalskiError::ConversationNotFound(conversation_id.to_string()))?;

        if let Some(role) = role {
            conversation.add_message("system", &role.get_prompt());

            if let Some(audience) = role.get_audience() {
                conversation.add_message("system", &audience.get_prompt());
            }
            if let Some(preset) = role.get_preset() {
                conversation.add_message("system", &preset.get_prompt());
            }
            if let Some(style) = role.get_style() {
                conversation.add_message("system", &style.get_prompt());
            }
        }

        let fallback_context = if use_memory && memory_context.is_empty() {
            Self::recent_conversation_context(&conversation.messages, 4)
        } else {
            String::new()
        };

        conversation.add_message("user", content);

        let model = conversation.model.clone();
        let mut messages = conversation.messages.clone();
        let effective_context = if !memory_context.is_empty() {
            memory_context
        } else {
            fallback_context
        };
        if !effective_context.is_empty() {
            let memory_prompt = format!(
                "Retrieved memory context (use only if relevant to the latest user request):{}",
                format!(
                    "\n--- Relevant Memories ---\n{}\n--- End Memories ---",
                    effective_context
                )
            );
            let insert_at = messages.len().saturating_sub(1);
            messages.insert(
                insert_at,
                Message {
                    role: "system".to_string(),
                    content: memory_prompt,
                    tool_calls: None,
                },
            );
        }
        let llm = self.llm_provider.clone();
        Ok((model, messages, llm))
    }

    /// Like [`Agent::chat_with_tools`] but emits **token deltas** over `token_tx` only for the first
    /// LLM completion **after at least one tool execution** in this request (final natural answer).
    pub async fn chat_with_tools_with_options(
        &mut self,
        conversation_id: &str,
        user_input: &str,
        use_memory: bool,
    ) -> Result<String, KowalskiError> {
        let mut final_response = String::new();
        let mut current_input = user_input.to_string();
        let mut iteration_count = 0;
        const MAX_ITERATIONS: usize = 5;
        let mut last_tool_call: Option<(String, serde_json::Value)> = None;
        let mut tool_parse_hint_sent = false;

        while iteration_count < MAX_ITERATIONS {
            iteration_count += 1;
            let response_text = self
                .chat_with_history_with_options(conversation_id, &current_input, None, use_memory)
                .await?;

            if repl_trace::repl_trace_enabled() {
                println!("[agent] {}", response_text);
            } else {
                println!("{}", response_text);
            }
            io::stdout()
                .flush()
                .map_err(|e| KowalskiError::Server(e.to_string()))?;

            let buffer = response_text.clone();
            let tool_calls = crate::utils::json::extract_tool_calls(&buffer);

            if !tool_calls.is_empty() {
                let tool_call = &tool_calls[0];
                let tool_call_key = (tool_call.name.clone(), tool_call.parameters.clone());
                if let Some(last) = &last_tool_call {
                    if *last == tool_call_key {
                        break;
                    }
                }
                last_tool_call = Some(tool_call_key);

                let tool_result = match self
                    .execute_tool(&tool_call.name, &tool_call.parameters)
                    .await
                {
                    Ok(output) => output.result.to_string(),
                    Err(e) => format!("{}", e),
                };

                let tool_message = format!("Tool result for {}: {}", tool_call.name, tool_result);
                self.add_message(conversation_id, "assistant", &tool_message)
                    .await;
                current_input = format!("Based on the tool result: {}", tool_result);
                continue;
            }

            if crate::utils::json::looks_like_tool_json_attempt(&buffer) && !tool_parse_hint_sent {
                tool_parse_hint_sent = true;
                self.add_message(conversation_id, "assistant", &buffer).await;
                const HINT: &str = "Your previous reply appeared to include a tool call but it could not be parsed as JSON. Reply with a single JSON object only: {\"name\": \"<tool_name>\", \"parameters\": { ... } } matching the available tools. No markdown fences or extra text.";
                current_input = HINT.to_string();
                continue;
            }

            final_response = buffer;
            self.add_message(conversation_id, "assistant", &final_response)
                .await;
            break;
        }

        Ok(final_response)
    }

    pub async fn chat_with_tools_stream_final(
        &mut self,
        conversation_id: &str,
        user_input: &str,
        token_tx: &tokio::sync::mpsc::Sender<String>,
    ) -> Result<String, KowalskiError> {
        self.chat_with_tools_stream_final_with_options(conversation_id, user_input, token_tx, true)
            .await
    }

    pub async fn chat_with_tools_stream_final_with_options(
        &mut self,
        conversation_id: &str,
        user_input: &str,
        token_tx: &tokio::sync::mpsc::Sender<String>,
        use_memory: bool,
    ) -> Result<String, KowalskiError> {
        let mut final_response = String::new();
        let mut current_input = user_input.to_string();
        let mut iteration_count = 0;
        const MAX_ITERATIONS: usize = 5;
        let mut last_tool_call: Option<(String, serde_json::Value)> = None;
        let mut tool_parse_hint_sent = false;
        // After a tool ran, the next LLM completion is streamed (final answer in the common case).
        let mut stream_next_llm_turn = false;

        debug!(
            "chat_with_tools_stream_final for input: '{}'",
            user_input
        );

        while iteration_count < MAX_ITERATIONS {
            iteration_count += 1;
            let use_stream = std::mem::replace(&mut stream_next_llm_turn, false);
            debug!(
                " === ITERATION {} (stream_final={}) ===",
                iteration_count, use_stream
            );

            let response_text = if use_stream {
                let (model, messages, llm) = self
                    .prepare_stream_turn_with_options(conversation_id, &current_input, None, use_memory)
                    .await?;
                let mut full = String::new();
                let mut stream = llm.chat_stream(&model, messages);
                while let Some(item) = stream.next().await {
                    let delta = item?;
                    if !delta.is_empty() {
                        full.push_str(&delta);
                        let _ = token_tx.send(delta).await;
                    }
                }
                full
            } else {
                self.chat_with_history_with_options(conversation_id, &current_input, None, use_memory)
                    .await?
            };

            if repl_trace::repl_trace_enabled() {
                println!("[agent] {}", response_text);
            } else {
                println!("{}", response_text);
            }
            io::stdout()
                .flush()
                .map_err(|e| KowalskiError::Server(e.to_string()))?;

            let buffer = response_text.clone();
            let tool_calls = crate::utils::json::extract_tool_calls(&buffer);

            if !tool_calls.is_empty() {
                let tool_call = &tool_calls[0];
                let tool_call_key = (tool_call.name.clone(), tool_call.parameters.clone());
                if let Some(last) = &last_tool_call {
                    if *last == tool_call_key {
                        debug!("Repeated tool call; breaking");
                        break;
                    }
                }
                last_tool_call = Some(tool_call_key.clone());

                if repl_trace::repl_trace_enabled() {
                    let params = serde_json::to_string(&tool_call.parameters)
                        .unwrap_or_else(|_| "{}".to_string());
                    println!("[tool] {} {}", tool_call.name, params);
                }

                let tool_result = match self
                    .execute_tool(&tool_call.name, &tool_call.parameters)
                    .await
                {
                    Ok(output) => output.result.to_string(),
                    Err(e) => format!("{}", e),
                };

                let tool_message = format!("Tool result for {}: {}", tool_call.name, tool_result);
                self.add_message(conversation_id, "assistant", &tool_message)
                    .await;

                current_input = format!("Based on the tool result: {}", tool_result);
                stream_next_llm_turn = true;
                continue;
            }

            if crate::utils::json::looks_like_tool_json_attempt(&buffer) && !tool_parse_hint_sent {
                tool_parse_hint_sent = true;
                warn!(
                    "Tool call JSON parse failed; requesting self-correction (non-stream)"
                );
                self.add_message(conversation_id, "assistant", &buffer)
                    .await;
                const HINT: &str = "Your previous reply appeared to include a tool call but it could not be parsed as JSON. Reply with a single JSON object only: {\"name\": \"<tool_name>\", \"parameters\": { ... } } matching the available tools. No markdown fences or extra text.";
                current_input = HINT.to_string();
                stream_next_llm_turn = false;
                continue;
            }

            final_response = buffer;
            self.add_message(conversation_id, "assistant", &final_response)
                .await;

            if let Some(tool_call) = rule_based_tool_call(user_input) {
                let tool_result_str = match self
                    .execute_tool(&tool_call.name, &tool_call.parameters)
                    .await
                {
                    Ok(output) => output.result.to_string(),
                    Err(e) => format!("Tool execution failed: {}", e),
                };
                self.add_message(conversation_id, "assistant", &tool_result_str)
                    .await;
                return Ok(tool_result_str);
            }

            break;
        }

        if iteration_count >= MAX_ITERATIONS {
            warn!("Reached maximum iterations (stream_final)");
        }

        Ok(final_response)
    }
}

#[async_trait]
impl Agent for BaseAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        crate::db::run_memory_migrations_if_configured(&config).await?;

        let llm_provider = crate::llm::create_llm_provider(&config)?;

        // Create memory providers
        let working_memory = std::sync::Arc::new(tokio::sync::Mutex::new(WorkingMemory::new(100)))
            as std::sync::Arc<tokio::sync::Mutex<dyn MemoryProvider + Send + Sync>>;

        let episodic_memory = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::memory::episodic::EpisodicBuffer::open(&config.memory, llm_provider.clone())
                .await?,
        ))
            as std::sync::Arc<tokio::sync::Mutex<dyn MemoryProvider + Send + Sync>>;

        let semantic_memory =
            crate::memory::helpers::create_semantic_memory(&config, llm_provider.clone()).await?;

        Self::new(
            config,
            "Base Agent",
            "A basic agent implementation",
            llm_provider,
            working_memory,
            episodic_memory,
            semantic_memory,
            crate::tools::manager::ToolManager::new(),
        )
        .await
    }

    fn start_conversation(&mut self, model: &str) -> String {
        info!("Starting conversation with model: {}", model);
        let conversation = Conversation::new(model);
        let id = conversation.id.clone();
        self.conversations.insert(id.clone(), conversation);
        id
    }

    fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.conversations.get(id)
    }

    fn list_conversations(&self) -> Vec<&Conversation> {
        self.conversations.values().collect()
    }

    fn delete_conversation(&mut self, id: &str) -> bool {
        self.conversations.remove(id).is_some()
    }

    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<String, KowalskiError> {
        self.chat_with_history_with_options(conversation_id, content, role, true)
            .await
    }

    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<Message>, KowalskiError> {
        BaseAgent::process_stream_response(self, conversation_id, chunk).await
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        BaseAgent::add_message(self, conversation_id, role, content).await;
    }

    fn export_conversation(&self, id: &str) -> Result<String, KowalskiError> {
        BaseAgent::export_conversation(self, id)
    }

    fn import_conversation(&mut self, json_str: &str) -> Result<String, KowalskiError> {
        BaseAgent::import_conversation(self, json_str)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

}

impl BaseAgent {
    pub async fn chat_with_history_with_options(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
        use_memory: bool,
    ) -> Result<String, KowalskiError> {
        let memory_context = self.build_memory_context(content, use_memory).await;

        let conversation = self
            .conversations
            .get_mut(conversation_id)
            .ok_or_else(|| KowalskiError::ConversationNotFound(conversation_id.to_string()))?;

        if let Some(role) = role {
            conversation.add_message("system", &role.get_prompt());

            if let Some(audience) = role.get_audience() {
                conversation.add_message("system", &audience.get_prompt());
            }
            if let Some(preset) = role.get_preset() {
                conversation.add_message("system", &preset.get_prompt());
            }
            if let Some(style) = role.get_style() {
                conversation.add_message("system", &style.get_prompt());
            }
        }

        let fallback_context = if use_memory && memory_context.is_empty() {
            Self::recent_conversation_context(&conversation.messages, 4)
        } else {
            String::new()
        };

        // Persist raw user input in conversation history.
        conversation.add_message("user", content);

        // Build request-time LLM messages: conversation history + optional memory context.
        // Memory context is ephemeral (not persisted as conversation turns).
        let mut llm_messages = conversation.messages.clone();
        let effective_context = if !memory_context.is_empty() {
            memory_context
        } else {
            fallback_context
        };
        if !effective_context.is_empty() {
            let memory_prompt = format!(
                "Retrieved memory context (use only if relevant to the latest user request):{}",
                format!(
                    "\n--- Relevant Memories ---\n{}\n--- End Memories ---",
                    effective_context
                )
            );
            let insert_at = llm_messages.len().saturating_sub(1);
            llm_messages.insert(
                insert_at,
                Message {
                    role: "system".to_string(),
                    content: memory_prompt,
                    tool_calls: None,
                },
            );
        }

        // Delegate to LLM Provider
        let response = self
            .llm_provider
            .chat(&conversation.model, &llm_messages)
            .await?;

        Ok(response)
    }

    async fn process_stream_response(
        &mut self,
        _conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<Message>, KowalskiError> {
        let text = String::from_utf8(chunk.to_vec())
            .map_err(|e| KowalskiError::Server(format!("Invalid UTF-8: {}", e)))?;

        let stream_response: StreamResponse =
            serde_json::from_str(&text).map_err(KowalskiError::Json)?;

        if stream_response.done {
            return Ok(None);
        }

        Ok(Some(stream_response.message))
    }

    async fn execute_tool(
        &mut self,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) -> Result<ToolOutput, KowalskiError> {
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

        self.tool_manager.execute(tool_name, input).await
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        // 2. STORAGE: Archive the message to the episodic buffer
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let memory_unit = MemoryUnit {
            id: format!("{}-{}", conversation_id, timestamp),
            timestamp,
            content: format!("[{}] {}", role, content),
            embedding: None, // Embeddings are generated during consolidation
        };

        // Add to Tier 1 working memory
        if let Err(e) = self
            .working_memory
            .lock()
            .await
            .add(memory_unit.clone())
            .await
        {
            eprintln!("Failed to add to working memory: {}", e);
        }

        // Add to Tier 2 episodic buffer
        if let Err(e) = self.episodic_memory.lock().await.add(memory_unit).await {
            eprintln!("Failed to add to episodic memory: {}", e);
        }

        if let Some(conversation) = self.conversations.get_mut(conversation_id) {
            conversation.add_message(role, content);
        }
    }

    fn export_conversation(&self, id: &str) -> Result<String, KowalskiError> {
        let conversation = self
            .conversations
            .get(id)
            .ok_or_else(|| KowalskiError::ConversationNotFound(id.to_string()))?;
        serde_json::to_string(conversation).map_err(KowalskiError::Json)
    }

    fn import_conversation(&mut self, json_str: &str) -> Result<String, KowalskiError> {
        let conversation: crate::conversation::Conversation =
            serde_json::from_str(json_str).map_err(KowalskiError::Json)?;
        let id = conversation.id.clone();
        self.conversations.insert(id.clone(), conversation);
        Ok(id)
    }

}

#[async_trait]
pub trait MessageHandler: Send + Sync {
    type Message;
    type Error;

    async fn handle_message(&mut self, message: Self::Message) -> Result<(), Self::Error>;
}

fn rule_based_tool_call(user_input: &str) -> Option<ToolCall> {
    let input = user_input.to_lowercase();
    if input.contains("list") && input.contains("directory") {
        if let Some(path) = input.split_whitespace().find(|w| w.starts_with('/')) {
            return Some(ToolCall {
                name: "fs_tool".to_string(),
                parameters: json!({ "task": "list_dir", "path": path }),
                reasoning: Some("Rule-based: user asked to list a directory".to_string()),
            });
        }
    }
    if input.contains("first 10 lines") && input.contains(".csv") {
        if let Some(path) = input.split_whitespace().find(|w| w.ends_with(".csv")) {
            return Some(ToolCall {
                name: "fs_tool".to_string(),
                parameters: json!({ "task": "get_file_first_lines", "path": path, "num_lines": 10 }),
                reasoning: Some("Rule-based: user asked for first 10 lines of a CSV".to_string()),
            });
        }
    }
    // Add more rules as needed...
    None
}
