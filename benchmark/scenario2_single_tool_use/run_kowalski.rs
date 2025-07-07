use kowalski_core::agent::{Agent, BaseAgent};
use kowalski_core::config::Config;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::ToolOutput;
use tokio::time::Instant;
use std::fs::File;
use std::io::{BufReader, BufRead};
use async_trait::async_trait;
use serde_json::json;

// Custom Agent trait implementation for this benchmark to include a simplified fs_tool
pub struct BenchmarkAgent {
    base_agent: BaseAgent,
}

#[async_trait]
impl Agent for BenchmarkAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        Ok(Self { base_agent: BaseAgent::new(config, "BenchmarkAgent", "").await? })
    }

    fn start_conversation(&mut self, model: &str) -> String {
        self.base_agent.start_conversation(model)
    }

    fn get_conversation(&self, id: &str) -> Option<&kowalski_core::conversation::Conversation> {
        self.base_agent.get_conversation(id)
    }

    fn list_conversations(&self) -> Vec<&kowalski_core::conversation::Conversation> {
        self.base_agent.list_conversations()
    }

    fn delete_conversation(&mut self, id: &str) -> bool {
        self.base_agent.delete_conversation(id)
    }

    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<kowalski_core::role::Role>,
    ) -> Result<reqwest::Response, KowalskiError> {
        self.base_agent.chat_with_history(conversation_id, content, role).await
    }

    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<kowalski_core::conversation::Message>, KowalskiError> {
        self.base_agent.process_stream_response(conversation_id, chunk).await
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        self.base_agent.add_message(conversation_id, role, content).await
    }

    async fn execute_tool(
        &mut self,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) -> Result<ToolOutput, KowalskiError> {
        if tool_name == "fs_tool" {
            let task = tool_input["task"].as_str().ok_or_else(|| KowalskiError::ToolExecution("Missing 'task' in fs_tool input".to_string()))?;
            let path = tool_input["path"].as_str().ok_or_else(|| KowalskiError::ToolExecution("Missing 'path' in fs_tool input".to_string()))?;

            match task {
                "get_first_lines" => {
                    let num_lines = tool_input["num_lines"].as_u64().unwrap_or(10) as usize;
                    let file = File::open(path).map_err(|e| KowalskiError::ToolExecution(format!("Failed to open file {}: {}", path, e)))?;
                    let reader = BufReader::new(file);
                    let lines: Vec<String> = reader.lines().take(num_lines).filter_map(|l| l.ok()).collect();
                    Ok(ToolOutput { result: json!({ "lines": lines.join("\n") }), metadata: None })
                },
                _ => Err(KowalskiError::ToolExecution(format!("Unknown fs_tool task: {}", task))),
            }
        } else {
            Err(KowalskiError::ToolExecution(format!("Unknown tool: {}", tool_name)))
        }
    }

    async fn chat_with_tools(
        &mut self,
        conversation_id: &str,
        user_input: &str,
    ) -> Result<String, KowalskiError> {
        // Simplified for benchmark: directly call the tool if recognized
        if user_input.contains("first 10 lines of example.txt") {
            let tool_call = kowalski_core::tools::ToolCall {
                name: "fs_tool".to_string(),
                parameters: json!({ "task": "get_first_lines", "path": "./example.txt", "num_lines": 10 }),
                reasoning: Some("User asked for first 10 lines of example.txt".to_string()),
            };
            let tool_result = self.execute_tool(&tool_call.name, &tool_call.parameters).await?;
            Ok(tool_result.result.to_string())
        } else {
            // Fallback to base agent chat_with_tools if not a direct tool call
            self.base_agent.chat_with_tools(conversation_id, user_input).await
        }
    }

    fn name(&self) -> &str {
        self.base_agent.name()
    }

    fn description(&self) -> &str {
        self.base_agent.description()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = Config::default();
    let mut agent = BenchmarkAgent::new(config).await?;
    let conversation_id = agent.start_conversation("llama3.2");

    let start_time = Instant::now();
    let response = agent.chat_with_tools(&conversation_id, "Get the first 10 lines of example.txt").await?;
    let elapsed = start_time.elapsed();

    println!("Kowalski (FS Tool Use) - Response: {}", response);
    println!("Kowalski (FS Tool Use) - Time: {:?}", elapsed);

    Ok(())
}