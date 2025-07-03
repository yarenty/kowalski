use crate::agent::types::{ChatRequest, StreamResponse};
use crate::config::Config;
use crate::conversation::Conversation;
use crate::conversation::Message;
use crate::error::KowalskiError;
use crate::role::Role;
use crate::tools::{ToolCall, ToolOutput};
use async_trait::async_trait;
use futures::StreamExt;
use log::info;
use reqwest::Response;
use serde_json;
use serde_json::json;
use std::any::Any;
use std::collections::HashMap;
use std::io::{self, Write};

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

    /// Chats with history
    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<Response, KowalskiError>;

    /// Processes a stream response
    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<Message>, KowalskiError>;

    /// Adds a message to a conversation
    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str);

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
        use crate::tools::ToolCall;
        let mut final_response = String::new();
        let mut current_input = user_input.to_string();
        let mut iteration_count = 0;
        const MAX_ITERATIONS: usize = 5; // Prevent infinite loops
        let mut last_tool_call: Option<(String, serde_json::Value)> = None;

        println!(
            "[DEBUG] Starting chat_with_tools for input: '{}'",
            user_input
        );

        while iteration_count < MAX_ITERATIONS {
            iteration_count += 1;
            println!("[DEBUG] === ITERATION {} ===", iteration_count);
            println!("[DEBUG] Current input: '{}'", current_input);

            //TODO: remove me - ... need some sleep today
            // // Only add the user message on the first iteration
            // if first_iteration {
            //     self.add_message(conversation_id, "user", &current_input).await;
            //     println!("[DEBUG] Added user message to conversation");
            //     first_iteration = false;
            // }

            // Get response from LLM
            println!("[DEBUG] Calling LLM...");
            let response = self
                .chat_with_history(conversation_id, &current_input, None)
                .await?;

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            // Process streaming response
            println!("[DEBUG] Processing streaming response...");
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        if let Ok(Some(message)) =
                            self.process_stream_response(conversation_id, &bytes).await
                        {
                            if !message.content.is_empty() {
                                print!("{}", message.content);
                                io::stdout()
                                    .flush()
                                    .map_err(|e| KowalskiError::Server(e.to_string()))?;
                                buffer.push_str(&message.content);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("\nError: {}", e);
                        return Err(KowalskiError::Server(e.to_string()));
                    }
                }
            }
            println!(); // New line after response
            println!("[DEBUG] Full LLM response: '{}'", buffer);

            // Try to extract JSON from mixed text response
            println!(
                "[DEBUG] ❌ Failed to parse entire response as tool call, trying to extract JSON..."
            );
            if let Some(json_start) = buffer.find('{') {
                // Find the first valid JSON object only
                let mut brace_count = 0;
                let mut end_idx = None;
                for (i, c) in buffer[json_start..].char_indices() {
                    match c {
                        '{' => brace_count += 1,
                        '}' => {
                            brace_count -= 1;
                            if brace_count == 0 {
                                end_idx = Some(json_start + i);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(json_end) = end_idx {
                    let json_str = &buffer[json_start..=json_end];
                    println!("[DEBUG] Extracted first JSON object: {}", json_str);
                    if let Ok(tool_call) = serde_json::from_str::<ToolCall>(json_str) {
                        // Detect repeated tool calls
                        let tool_call_key = (tool_call.name.clone(), tool_call.parameters.clone());
                        if let Some(last) = &last_tool_call {
                            if *last == tool_call_key {
                                println!(
                                    "[DEBUG] Detected repeated tool call. Breaking loop to prevent infinite tool call loop."
                                );
                                break;
                            }
                        }
                        last_tool_call = Some(tool_call_key.clone());
                        println!("[DEBUG] ✅ Tool call successfully parsed from extracted JSON!");
                        println!("[DEBUG] Tool: {}", tool_call.name);
                        println!("[DEBUG] Parameters: {}", tool_call.parameters);
                        println!("[DEBUG] Reasoning: {:?}", tool_call.reasoning);
                        let tool_result = match self
                            .execute_tool(&tool_call.name, &tool_call.parameters)
                            .await
                        {
                            Ok(output) => output.result.to_string(),
                            Err(e) => {
                                let err_msg = format!("{}", e);
                                println!("[DEBUG] Tool execution failed: {}", err_msg);
                                // Tool chaining: if csv_tool is called with an unsupported task, try fs_tool get_file_contents then csv_tool process_csv
                                if tool_call.name == "csv_tool"
                                    && tool_call
                                        .parameters
                                        .get("task")
                                        .map(|v| v == "read_file")
                                        .unwrap_or(false)
                                {
                                    if let Some(path) =
                                        tool_call.parameters.get("path").and_then(|v| v.as_str())
                                    {
                                        println!(
                                            "[DEBUG] Tool chaining: Detected csv_tool with read_file. Chaining fs_tool get_file_contents then csv_tool process_csv."
                                        );
                                        // Step 1: fs_tool get_file_contents
                                        let fs_params = serde_json::json!({"task": "get_file_contents", "path": path});
                                        match self.execute_tool("fs_tool", &fs_params).await {
                                            Ok(fs_output) => {
                                                let file_contents = fs_output
                                                    .result
                                                    .get("contents")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("");
                                                // Step 2: csv_tool process_csv
                                                let csv_params = serde_json::json!({"task": "process_csv", "content": file_contents});
                                                match self
                                                    .execute_tool("csv_tool", &csv_params)
                                                    .await
                                                {
                                                    Ok(csv_output) => csv_output.result.to_string(),
                                                    Err(e2) => format!(
                                                        "Tool chaining failed at csv_tool: {}",
                                                        e2
                                                    ),
                                                }
                                            }
                                            Err(e1) => {
                                                format!("Tool chaining failed at fs_tool: {}", e1)
                                            }
                                        }
                                    } else {
                                        err_msg
                                    }
                                } else {
                                    err_msg
                                }
                            }
                        };
                        let tool_message =
                            format!("Tool result for {}: {}", tool_call.name, tool_result);
                        self.add_message(conversation_id, "assistant", &tool_message)
                            .await;
                        println!("[DEBUG] Added tool result to conversation");
                        current_input = format!("Based on the tool result: {}", tool_result);
                        println!("[DEBUG] Continuing with new input: '{}'", current_input);
                        continue;
                    } else {
                        println!("[DEBUG] ❌ Failed to parse extracted JSON as tool call");
                    }
                }
            }

            // Not a tool call, this is the final answer
            final_response = buffer;
            self.add_message(conversation_id, "assistant", &final_response)
                .await;
            println!("[DEBUG] ✅ Final response set: '{}'", final_response);

            if let Some(tool_call) = rule_based_tool_call(user_input) {
                println!("[DEBUG] Rule-based tool call triggered: {:?}", tool_call);
                let tool_result = self
                    .execute_tool(&tool_call.name, &tool_call.parameters)
                    .await;
                let tool_result_str = match tool_result {
                    Ok(output) => output.result.to_string(),
                    Err(e) => format!("Tool execution failed: {}", e),
                };
                self.add_message(conversation_id, "assistant", &tool_result_str)
                    .await;
                println!("[DEBUG] Rule-based tool result: {}", tool_result_str);
                return Ok(tool_result_str);
            }

            break;
        }

        if iteration_count >= MAX_ITERATIONS {
            println!("[WARNING] Reached maximum iterations, returning current response");
        }

        println!(
            "[DEBUG] chat_with_tools completed after {} iterations",
            iteration_count
        );
        Ok(final_response)
    }

    /// Gets the agent's name
    fn name(&self) -> &str;

    /// Gets the agent's description
    fn description(&self) -> &str;

    fn as_any(&self) -> &dyn Any;
}

use kowalski_memory::working::WorkingMemory;
use kowalski_memory::episodic::EpisodicBuffer;
use kowalski_memory::semantic::SemanticStore;

/// The base agent implementation that provides common functionality.
pub struct BaseAgent {
    pub client: reqwest::Client,
    pub config: Config,
    pub conversations: HashMap<String, Conversation>,
    pub name: String,
    pub description: String,
    pub system_prompt: Option<String>,
    // Memory Tiers
    pub working_memory: WorkingMemory,
    pub episodic_memory: EpisodicBuffer,
    pub semantic_memory: SemanticStore,
}

impl BaseAgent {
    pub async fn new(config: Config, name: &str, description: &str) -> Result<Self, KowalskiError> {
        let client = reqwest::ClientBuilder::new()
            .pool_max_idle_per_host(0)
            .build()
            .map_err(KowalskiError::Request)?;

        info!("BaseAgent created with name: {}", name);

        // Initialize memory tiers
        let working_memory = WorkingMemory::new(100); // Capacity of 100 units
        let episodic_memory = EpisodicBuffer::new("./db/episodic_buffer")
            .map_err(|e| KowalskiError::Initialization(format!("Failed to init episodic buffer: {}", e)))?;
        let semantic_memory = SemanticStore::new("http://localhost:6333") // Assuming Qdrant is running here
            .await
            .map_err(|e| KowalskiError::Initialization(format!("Failed to init semantic store: {}", e)))?;

        Ok(Self {
            client,
            config,
            conversations: HashMap::new(),
            name: name.to_string(),
            description: description.to_string(),
            system_prompt: None,
            working_memory,
            episodic_memory,
            semantic_memory,
        })
    }

    pub fn set_temperature(&mut self, temperature: f32) {
        self.config.chat.temperature = temperature;
    }

    pub fn set_system_prompt(&mut self, prompt: &str) {
        self.system_prompt = Some(prompt.to_string());
    }
}

#[async_trait]
impl Agent for BaseAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        Self::new(config, "Base Agent", "A basic agent implementation").await
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

    use kowalski_memory::MemoryProvider;

    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<Response, KowalskiError> {
        // 1. RECALL: Retrieve relevant memories before calling the LLM
        let memories = self.semantic_memory.retrieve(content).await.unwrap_or_default();
        let memory_context = if !memories.is_empty() {
            let concatenated_memories = memories
                .iter()
                .map(|m| m.content.as_str())
                .collect::<Vec<&str>>()
                .join("\n---\n");
            format!("\n--- Relevant Memories ---\n{}\n--- End Memories ---", concatenated_memories)
        } else {
            String::new()
        };

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

        // Inject memories into the user's message
        let content_with_memory = format!("{}{}", content, memory_context);
        conversation.add_message("user", &content_with_memory);

        let request = ChatRequest {
            model: conversation.model.clone(),
            messages: conversation.messages.clone(),
            stream: true,
            temperature: self.config.chat.temperature,
            max_tokens: self.config.chat.max_tokens as usize,
            tools: None,
        };

        let response = self
            .client
            .post(format!(
                "http://{}:{}/api/chat",
                self.config.ollama.host, self.config.ollama.port
            ))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(KowalskiError::Server(error_text));
        }

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

    use kowalski_memory::MemoryUnit;
use std::time::{SystemTime, UNIX_EPOCH};

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
        if let Err(e) = self.working_memory.add(memory_unit.clone()).await {
            eprintln!("Failed to add to working memory: {}", e);
        }

        // Add to Tier 2 episodic buffer
        if let Err(e) = self.episodic_memory.add(memory_unit).await {
            eprintln!("Failed to add to episodic memory: {}", e);
        }

        if let Some(conversation) = self.conversations.get_mut(conversation_id) {
            conversation.add_message(role, content);
        }
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
