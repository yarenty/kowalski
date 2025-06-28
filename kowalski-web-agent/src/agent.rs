use crate::config::WebAgentConfig;
use async_trait::async_trait;
use kowalski_agent_template::TemplateAgent;
use kowalski_agent_template::templates::general::GeneralTemplate;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::Conversation;
use kowalski_core::error::KowalskiError;
use kowalski_core::role::Role;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput};
use kowalski_tools::web::{WebScrapeTool, WebSearchTool};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{self, Write};
use tokio::io::AsyncReadExt;
use futures::StreamExt;

/// WebAgent: A specialized agent for web-related tasks
/// This agent is built on top of the TemplateAgent and provides web-specific functionality
pub struct WebAgent {
    agent: TemplateAgent,
    config: WebAgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResult {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub input: String,
    pub reasoning: Option<String>,
}

impl WebAgent {
    /// Creates a new WebAgent with the specified configuration
    pub async fn new(_config: Config) -> Result<Self, KowalskiError> {
        // TODO: Convert Config to WebAgentConfig if needed
        let web_config = WebAgentConfig::default();
        let provider = web_config.search.default_provider.to_string();
        let search_tool = WebSearchTool::new(provider);
        let scrape_tool = WebScrapeTool::new();
        let tools: Vec<Box<dyn Tool + Send + Sync>> =
            vec![Box::new(search_tool), Box::new(scrape_tool)];
        
        // Enhanced system prompt that explicitly encourages tool usage
        let system_prompt = r#"You are a web research assistant specialized in finding and analyzing online information. 

AVAILABLE TOOLS:
1. web_search - Search the web for information. Use this when you need to find current information, news, or general knowledge.
2. web_scrape - Scrape content from a specific URL. Use this when you have a URL and need to extract its content.

TOOL USAGE INSTRUCTIONS:
- ALWAYS use tools when asked about current events, recent information, or anything that requires up-to-date data
- For general queries, start with web_search to find relevant URLs
- When you have a specific URL, use web_scrape to get detailed content
- You can chain tools: search first, then scrape interesting URLs

RESPONSE FORMAT:
When you need to use a tool, respond with JSON in this exact format:
{
  "tool": "web_search",
  "input": "your search query here",
  "reasoning": "why you're using this tool"
}

or

{
  "tool": "web_scrape", 
  "input": "https://example.com",
  "reasoning": "why you're scraping this URL"
}

When you have a final answer, respond normally without JSON formatting.

Remember: Use tools proactively to provide accurate, up-to-date information!"#.to_string();

        let builder = GeneralTemplate::create_agent(
            tools,
            Some(system_prompt),
            Some(0.7),
        )
        .await
        .map_err(|e| KowalskiError::Configuration(e.to_string()))?;
        let agent = builder.build().await?;
        Ok(Self {
            agent,
            config: web_config,
        })
    }

    /// Chat with the web agent using ReAct-style tool calling
    pub async fn chat_with_tools(&mut self, conversation_id: &str, user_input: &str) -> Result<String, KowalskiError> {
        let mut final_response = String::new();
        let mut current_input = user_input.to_string();
        let mut iteration_count = 0;
        const MAX_ITERATIONS: usize = 5; // Prevent infinite loops

        println!("[DEBUG] Starting chat_with_tools for input: '{}'", user_input);

        while iteration_count < MAX_ITERATIONS {
            iteration_count += 1;
            println!("[DEBUG] === ITERATION {} ===", iteration_count);
            println!("[DEBUG] Current input: '{}'", current_input);

            // Add user input to conversation
            self.add_message(conversation_id, "user", &current_input).await;
            println!("[DEBUG] Added user message to conversation");

            // Get response from LLM
            println!("[DEBUG] Calling LLM...");
            let response = self
                .agent.base_mut().chat_with_history(conversation_id, &current_input, None)
                .await?;

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            // Process streaming response
            println!("[DEBUG] Processing streaming response...");
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        if let Ok(Some(message)) = self.process_stream_response(conversation_id, &bytes).await {
                            if !message.content.is_empty() {
                                print!("{}", message.content);
                                io::stdout().flush().map_err(|e| KowalskiError::Server(e.to_string()))?;
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

            // Try to parse as tool call
            println!("[DEBUG] Attempting to parse as tool call...");
            
            // First try to parse the entire response as JSON
            if let Ok(tool_call) = serde_json::from_str::<ToolCall>(&buffer) {
                println!("[DEBUG] ✅ Tool call successfully parsed!");
                println!("[DEBUG] Tool: {}", tool_call.tool);
                println!("[DEBUG] Input: {}", tool_call.input);
                println!("[DEBUG] Reasoning: {:?}", tool_call.reasoning);
                
                // Execute the tool using specialized methods
                let tool_result = match tool_call.tool.as_str() {
                    "web_search" => {
                        println!("[DEBUG] Executing web_search with query: '{}'", tool_call.input);
                        match self.search(&tool_call.input).await {
                            Ok(results) => {
                                let result_str = if results.is_empty() {
                                    "No search results found.".to_string()
                                } else {
                                    let mut formatted = String::new();
                                    for (i, result) in results.iter().enumerate() {
                                        formatted.push_str(&format!("{}. {}\n   URL: {}\n   Snippet: {}\n\n", 
                                            i + 1, result.title, result.url, result.snippet));
                                    }
                                    formatted
                                };
                                println!("[DEBUG] ✅ web_search completed successfully");
                                println!("[DEBUG] Search results: {}", result_str);
                                result_str
                            }
                            Err(e) => {
                                println!("[DEBUG] ❌ web_search failed: {}", e);
                                format!("Search failed: {}", e)
                            }
                        }
                    }
                    "web_scrape" => {
                        println!("[DEBUG] Executing web_scrape with URL: '{}'", tool_call.input);
                        match self.fetch_page(&tool_call.input).await {
                            Ok(page) => {
                                let result_str = format!("Page Title: {}\n\nContent: {}", page.title, page.content);
                                println!("[DEBUG] ✅ web_scrape completed successfully");
                                println!("[DEBUG] Scraped content length: {} chars", page.content.len());
                                result_str
                            }
                            Err(e) => {
                                println!("[DEBUG] ❌ web_scrape failed: {}", e);
                                format!("Scraping failed: {}", e)
                            }
                        }
                    }
                    _ => {
                        println!("[DEBUG] Unknown tool: {}, using generic execution", tool_call.tool);
                        self.execute_tool(&tool_call.tool, &tool_call.input).await?
                    }
                };
                
                // Add tool result to conversation
                let tool_message = format!("Tool result for {}: {}", tool_call.tool, tool_result);
                self.add_message(conversation_id, "assistant", &tool_message).await;
                println!("[DEBUG] Added tool result to conversation");
                
                // Continue loop with tool result as next input
                current_input = format!("Based on the tool result: {}", tool_result);
                println!("[DEBUG] Continuing with new input: '{}'", current_input);
                continue;
            } else {
                // Try to extract JSON from mixed text response
                println!("[DEBUG] ❌ Failed to parse entire response as tool call, trying to extract JSON...");
                
                // Look for JSON blocks in the response
                if let Some(json_start) = buffer.find('{') {
                    if let Some(json_end) = buffer.rfind('}') {
                        let json_str = &buffer[json_start..=json_end];
                        println!("[DEBUG] Extracted JSON: {}", json_str);
                        
                        if let Ok(tool_call) = serde_json::from_str::<ToolCall>(json_str) {
                            println!("[DEBUG] ✅ Tool call successfully parsed from extracted JSON!");
                            println!("[DEBUG] Tool: {}", tool_call.tool);
                            println!("[DEBUG] Input: {}", tool_call.input);
                            println!("[DEBUG] Reasoning: {:?}", tool_call.reasoning);
                            
                            // Execute the tool using specialized methods
                            let tool_result = match tool_call.tool.as_str() {
                                "web_search" => {
                                    println!("[DEBUG] Executing web_search with query: '{}'", tool_call.input);
                                    match self.search(&tool_call.input).await {
                                        Ok(results) => {
                                            let result_str = if results.is_empty() {
                                                "No search results found.".to_string()
                                            } else {
                                                let mut formatted = String::new();
                                                for (i, result) in results.iter().enumerate() {
                                                    formatted.push_str(&format!("{}. {}\n   URL: {}\n   Snippet: {}\n\n", 
                                                        i + 1, result.title, result.url, result.snippet));
                                                }
                                                formatted
                                            };
                                            println!("[DEBUG] ✅ web_search completed successfully");
                                            println!("[DEBUG] Search results: {}", result_str);
                                            result_str
                                        }
                                        Err(e) => {
                                            println!("[DEBUG] ❌ web_search failed: {}", e);
                                            format!("Search failed: {}", e)
                                        }
                                    }
                                }
                                "web_scrape" => {
                                    println!("[DEBUG] Executing web_scrape with URL: '{}'", tool_call.input);
                                    match self.fetch_page(&tool_call.input).await {
                                        Ok(page) => {
                                            let result_str = format!("Page Title: {}\n\nContent: {}", page.title, page.content);
                                            println!("[DEBUG] ✅ web_scrape completed successfully");
                                            println!("[DEBUG] Scraped content length: {} chars", page.content.len());
                                            result_str
                                        }
                                        Err(e) => {
                                            println!("[DEBUG] ❌ web_scrape failed: {}", e);
                                            format!("Scraping failed: {}", e)
                                        }
                                    }
                                }
                                _ => {
                                    println!("[DEBUG] Unknown tool: {}, using generic execution", tool_call.tool);
                                    self.execute_tool(&tool_call.tool, &tool_call.input).await?
                                }
                            };
                            
                            // Add tool result to conversation
                            let tool_message = format!("Tool result for {}: {}", tool_call.tool, tool_result);
                            self.add_message(conversation_id, "assistant", &tool_message).await;
                            println!("[DEBUG] Added tool result to conversation");
                            
                            // Continue loop with tool result as next input
                            current_input = format!("Based on the tool result: {}", tool_result);
                            println!("[DEBUG] Continuing with new input: '{}'", current_input);
                            continue;
                        } else {
                            println!("[DEBUG] ❌ Failed to parse extracted JSON as tool call");
                        }
                    }
                }
                
                println!("[DEBUG] ❌ No valid tool call found - treating as final response");
            }

            // Not a tool call, this is the final answer
            final_response = buffer;
            self.add_message(conversation_id, "assistant", &final_response).await;
            println!("[DEBUG] ✅ Final response set: '{}'", final_response);
            break;
        }

        if iteration_count >= MAX_ITERATIONS {
            println!("[WARNING] Reached maximum iterations, returning current response");
        }

        println!("[DEBUG] chat_with_tools completed after {} iterations", iteration_count);
        Ok(final_response)
    }

    /// Execute a specific tool
    async fn execute_tool(&self, tool_name: &str, input: &str) -> Result<String, KowalskiError> {
        let mut tools = self.agent.tool_chain.write().await;
        
        if let Some(tool) = tools.iter_mut().find(|t| t.name() == tool_name) {
            let tool_input = ToolInput::new(
                tool_name.to_string(),
                input.to_string(),
                json!({"query": input}),
            );
            
            match tool.execute(tool_input).await {
                Ok(output) => {
                    let result = format!("{}", output.result);
                    println!("[DEBUG] Tool {} executed successfully: {}", tool_name, result);
                    Ok(result)
                }
                Err(e) => {
                    let error_msg = format!("Tool {} failed: {}", tool_name, e);
                    println!("[DEBUG] {}", error_msg);
                    Err(KowalskiError::ToolExecution(error_msg))
                }
            }
        } else {
            let error_msg = format!("Tool {} not found", tool_name);
            println!("[DEBUG] {}", error_msg);
            Err(KowalskiError::ToolExecution(error_msg))
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, KowalskiError> {
        use serde_json::Value;
        println!("[DEBUG] 🔍 Starting web search for query: '{}'", query);
        
        let mut tools = self.agent.tool_chain.write().await;
        println!("[DEBUG] Available tools: {:?}", tools.iter().map(|t| t.name()).collect::<Vec<_>>());
        
        let tool = tools.iter_mut().find(|t| t.name() == "web_search");
        let tool = match tool {
            Some(t) => {
                println!("[DEBUG] ✅ Found web_search tool");
                t
            }
            None => {
                println!("[DEBUG] ❌ web_search tool not found!");
                return Err(KowalskiError::ToolExecution(
                    "web_search tool not found".to_string(),
                ));
            }
        };
        
        let input = ToolInput::new(
            "search".to_string(),
            query.to_string(),
            serde_json::json!({"query": query}),
        );
        println!("[DEBUG] Executing web_search tool with input: {:?}", input);
        
        let output = tool.execute(input).await?;
        let results_val = &output.result;

        println!("[DEBUG] Raw tool output: {}", &results_val);
        println!("[DEBUG] Output type: {:?}", std::any::type_name_of_val(&*results_val));

        // DuckDuckGo-specific parsing
        if let Some(provider) = results_val.get("provider").and_then(|v| v.as_str()) {
            println!("[DEBUG] Provider detected: {}", provider);
            if provider == "duckduckgo" {
                if let Some(raw) = results_val.get("results").and_then(|v| v.as_str()) {
                    println!("[DEBUG] Raw DuckDuckGo results: {}", raw);
                    let parsed: Result<Value, _> = serde_json::from_str(raw);
                    if let Ok(json) = parsed {
                        println!("[DEBUG] Successfully parsed DuckDuckGo JSON");
                        // Try to extract RelatedTopics (and nested Topics)
                        let mut results = Vec::new();
                        if let Some(related) = json.get("RelatedTopics").and_then(|v| v.as_array())
                        {
                            println!("[DEBUG] Found {} RelatedTopics", related.len());
                            for item in related {
                                // If item has "Topics", it's a category, else it's a result
                                if let Some(topics) = item.get("Topics").and_then(|v| v.as_array())
                                {
                                    println!("[DEBUG] Found {} Topics in category", topics.len());
                                    for topic in topics {
                                        if let (Some(title), Some(url), Some(snippet)) = (
                                            topic.get("Text").and_then(|v| v.as_str()),
                                            topic.get("FirstURL").and_then(|v| v.as_str()),
                                            topic.get("Text").and_then(|v| v.as_str()),
                                        ) {
                                            results.push(SearchResult {
                                                title: title.to_string(),
                                                url: url.to_string(),
                                                snippet: snippet.to_string(),
                                            });
                                        }
                                    }
                                } else if let (Some(title), Some(url), Some(snippet)) = (
                                    item.get("Text").and_then(|v| v.as_str()),
                                    item.get("FirstURL").and_then(|v| v.as_str()),
                                    item.get("Text").and_then(|v| v.as_str()),
                                ) {
                                    results.push(SearchResult {
                                        title: title.to_string(),
                                        url: url.to_string(),
                                        snippet: snippet.to_string(),
                                    });
                                }
                            }
                        }
                        // Fallback: If no results, try to use Heading/AbstractURL
                        if results.is_empty() {
                            println!("[DEBUG] No RelatedTopics found, trying fallback parsing");
                            let title = json
                                .get("Heading")
                                .and_then(|v| v.as_str())
                                .unwrap_or(query);
                            let url = json
                                .get("AbstractURL")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let snippet = json
                                .get("AbstractText")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if !title.is_empty() || !url.is_empty() || !snippet.is_empty() {
                                results.push(SearchResult {
                                    title: title.to_string(),
                                    url: url.to_string(),
                                    snippet: snippet.to_string(),
                                });
                            }
                        }
                        // If still empty, fallback to query
                        if results.is_empty() {
                            println!("[DEBUG] Still no results, using query as fallback");
                            results.push(SearchResult {
                                title: query.to_string(),
                                url: String::new(),
                                snippet: String::new(),
                            });
                        }
                        println!("[DEBUG] ✅ Returning {} search results", results.len());
                        return Ok(results);
                    } else {
                        println!("[DEBUG] ❌ Failed to parse DuckDuckGo JSON: {:?}", parsed);
                    }
                } else {
                    println!("[DEBUG] ❌ No 'results' field found in DuckDuckGo response");
                }
            }
        } else {
            println!("[DEBUG] No provider field found, trying generic parsing");
        }
        
        // Default: try to parse as array of SearchResult
        let results: Vec<SearchResult> = if results_val.is_array() {
            println!("[DEBUG] Trying to parse as array");
            serde_json::from_value(results_val.clone()).unwrap_or_default()
        } else if results_val.is_string() {
            println!("[DEBUG] Trying to parse as string");
            let raw = results_val.as_str().unwrap();
            serde_json::from_str(raw).unwrap_or_default()
        } else {
            println!("[DEBUG] ❌ Could not parse results, returning empty");
            Vec::new()
        };
        
        println!("[DEBUG] ✅ Final search results: {} items", results.len());
        Ok(results)
    }

    pub async fn fetch_page(&self, url: &str) -> Result<PageResult, KowalskiError> {
        use serde_json::Value;
        println!("[DEBUG] 🌐 Starting web scrape for URL: '{}'", url);
        
        let mut tools = self.agent.tool_chain.write().await;
        println!("[DEBUG] Available tools: {:?}", tools.iter().map(|t| t.name()).collect::<Vec<_>>());
        
        let tool = tools.iter_mut().find(|t| t.name() == "web_scrape");
        let tool = match tool {
            Some(t) => {
                println!("[DEBUG] ✅ Found web_scrape tool");
                t
            }
            None => {
                println!("[DEBUG] ❌ web_scrape tool not found!");
                return Err(KowalskiError::ToolExecution(
                    "web_scrape tool not found".to_string(),
                ));
            }
        };
        
        let input = ToolInput::new(
            "scrape_static".to_string(),
            url.to_string(),
            serde_json::json!({"url": url, "selectors": ["title", "body"]}),
        );
        println!("[DEBUG] Executing web_scrape tool with input: {:?}", input);
        
        let output = tool.execute(input).await?;
        println!("[DEBUG] Raw tool output: {}", &output.result);
        
        // Parse the first result as the page title/content
        let arr = output.result.as_array().cloned().unwrap_or_default();
        println!("[DEBUG] Parsed array has {} elements", arr.len());
        
        let title = arr
            .iter()
            .find(|v| v["selector"] == "title")
            .and_then(|v| v["text"].as_str())
            .unwrap_or("")
            .to_string();
        let content = arr
            .iter()
            .find(|v| v["selector"] == "body")
            .and_then(|v| v["text"].as_str())
            .unwrap_or("")
            .to_string();
            
        println!("[DEBUG] ✅ Extracted title: '{}' ({} chars)", title, title.len());
        println!("[DEBUG] ✅ Extracted content: {} chars", content.len());
        
        Ok(PageResult { title, content })
    }
}

#[async_trait]
impl Agent for WebAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        WebAgent::new(config).await
    }

    fn start_conversation(&mut self, model: &str) -> String {
        let conv_id = self.agent.base_mut().start_conversation(model);
        
        // Add the system prompt to the conversation to ensure the LLM knows about tools
        let system_prompt = r#"You are a web research assistant specialized in finding and analyzing online information. 

AVAILABLE TOOLS:
1. web_search - Search the web for information. Use this when you need to find current information, news, or general knowledge.
2. web_scrape - Scrape content from a specific URL. Use this when you have a URL and need to extract its content.

TOOL USAGE INSTRUCTIONS:
- ALWAYS use tools when asked about current events, recent information, or anything that requires up-to-date data
- For general queries, start with web_search to find relevant URLs
- When you have a specific URL, use web_scrape to get detailed content
- You can chain tools: search first, then scrape interesting URLs

RESPONSE FORMAT:
When you need to use a tool, respond with JSON in this exact format:
{
  "tool": "web_search",
  "input": "your search query here",
  "reasoning": "why you're using this tool"
}

or

{
  "tool": "web_scrape", 
  "input": "https://example.com",
  "reasoning": "why you're scraping this URL"
}

When you have a final answer, respond normally without JSON formatting.

Remember: Use tools proactively to provide accurate, up-to-date information!"#;
        
        // Add the system prompt to the conversation synchronously
        if let Some(conversation) = self.agent.base_mut().conversations.get_mut(&conv_id) {
            conversation.add_message("system", system_prompt);
        }
        
        conv_id
    }

    fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.agent.base().get_conversation(id)
    }

    fn list_conversations(&self) -> Vec<&Conversation> {
        self.agent.base().list_conversations()
    }

    fn delete_conversation(&mut self, id: &str) -> bool {
        self.agent.base_mut().delete_conversation(id)
    }

    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<reqwest::Response, KowalskiError> {
        println!("[DEBUG] WebAgent chat_with_history called with content: '{}'", content);
        
        // Use the enhanced tool-calling method for web agents
        match self.chat_with_tools(conversation_id, content).await {
            Ok(response_text) => {
                println!("[DEBUG] chat_with_tools completed successfully, response length: {}", response_text.len());
                // For now, just print the response and fall back to regular chat
                // This is a workaround since we can't easily create a streaming response
                println!("[DEBUG] Tool-calling response: {}", response_text);
                // Fallback to regular chat for now
                self.agent
                    .base_mut()
                    .chat_with_history(conversation_id, content, role)
                    .await
            }
            Err(e) => {
                println!("[DEBUG] chat_with_tools failed: {}, falling back to regular chat", e);
                // Fallback to regular chat
                self.agent
                    .base_mut()
                    .chat_with_history(conversation_id, content, role)
                    .await
            }
        }
    }

    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<kowalski_core::conversation::Message>, KowalskiError> {
        self.agent
            .base_mut()
            .process_stream_response(conversation_id, chunk)
            .await
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        self.agent
            .base_mut()
            .add_message(conversation_id, role, content)
            .await;
    }

    fn name(&self) -> &str {
        self.agent.base().name()
    }

    fn description(&self) -> &str {
        self.agent.base().description()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kowalski_core::config::Config;

    #[tokio::test]
    async fn test_web_agent_creation() {
        let config = Config::default();
        let agent = WebAgent::new(config).await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_web_agent_conversation() {
        let config = Config::default();
        let mut agent = WebAgent::new(config).await.unwrap();
        let conv_id = agent.start_conversation("test-model");
        assert!(!conv_id.is_empty());
    }
}
