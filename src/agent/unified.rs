use async_trait::async_trait;
use reqwest::Response;
use crate::config::Config;
use crate::conversation::Conversation;
use crate::role::Role;
use super::{Agent, BaseAgent};
use crate::tools::{ToolChain, ToolType, SearchTool, TaskType};
use crate::tools::SearchProvider;
use crate::tools::WebBrowser;
use crate::tools::WebScraper;
use log::{debug, info};
use std::collections::HashMap;
use serde_json;
use crate::agent::types::Message;
use crate::utils::KowalskiError;

/// UnifiedAgent: A versatile agent that combines chat capabilities with tool usage
pub struct UnifiedAgent {
    base: BaseAgent,
    tool_chain: ToolChain,
}

#[async_trait]
impl Agent for UnifiedAgent {
    fn new(config: Config) -> Result<Self, KowalskiError> {
        let base = BaseAgent::new(
            config.clone(),
            "UnifiedAgent",
            "A versatile agent that combines chat capabilities with dynamic tool usage",
        )?;

        let mut tool_chain = ToolChain::new();
        
        let search_tool = SearchTool::new(
            SearchProvider::DuckDuckGo,
            config.search.api_key.as_ref().unwrap_or(&String::new()).clone(),
        );
        tool_chain.add_tool(
            ToolType::Search(search_tool.clone()),
            vec![TaskType::Search]
        );
        
        tool_chain.add_tool(
            ToolType::Browser(WebBrowser::new(config.clone())),
            vec![TaskType::BrowseDynamic]
        );
        
        tool_chain.add_tool(
            ToolType::Scraper(WebScraper::new()),
            vec![TaskType::ScrapStatic]
        );

        Ok(Self { base, tool_chain })
    }

    fn start_conversation(&mut self, model: &str) -> String {
        self.base.start_conversation(model)
    }

    fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.base.get_conversation(id)
    }

    fn list_conversations(&self) -> Vec<&Conversation> {
        self.base.list_conversations()
    }

    fn delete_conversation(&mut self, id: &str) -> bool {
        self.base.delete_conversation(id)
    }

    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<Response, KowalskiError> {
        debug!("Processing chat request: {}", content);

        // Process content through tool chain if needed
        let processed_content = if self.should_use_tools(content) {
            info!("Using tools to process: {}", content);
            let tool_input = crate::tools::ToolInput::new(content.to_string());
            let tool_output = self.tool_chain.execute(tool_input).await?;
            tool_output.content
        } else {
            content.to_string()
        };

        let conversation = self.base.conversations.get_mut(conversation_id)
            .ok_or_else(|| KowalskiError::ConversationNotFound(conversation_id.to_string()))?;

        // Add system messages based on role if provided
        if let Some(role) = role {
            conversation.add_message("system", role.get_prompt());
            
            if let Some(audience) = role.get_audience() {
                conversation.add_message("system", audience.get_prompt());
            }
            if let Some(preset) = role.get_preset() {
                conversation.add_message("system", preset.get_prompt());
            }
            if let Some(style) = role.get_style() {
                conversation.add_message("system", style.get_prompt());
            }
        }

        conversation.add_message("user", &processed_content);

        let request = super::types::ChatRequest {
            model: conversation.model.clone(),
            messages: conversation.messages.clone(),
            stream: true,
            temperature: self.base.config.chat.temperature.unwrap_or(0.7),
            max_tokens: self.base.config.chat.max_tokens.unwrap_or(2048) as usize,
            tools: Some(vec![
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": "search",
                        "description": "Search the web for information",
                        "parameters": {
                            "type": "object",
                            "required": ["query"],
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "The search query"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "The maximum number of results to return"
                                },
                                "include_images": {
                                    "type": "boolean",
                                    "description": "Whether to include images in the search results"
                                },
                                "include_videos": {
                                    "type": "boolean",
                                    "description": "Whether to include videos in the search results"
                                },
                                "include_news": {
                                    "type": "boolean",
                                    "description": "Whether to include news in the search results"
                                },
                                "include_maps": {
                                    "type": "boolean",
                                    "description": "Whether to include maps in the search results"
                                },
                                "use_cache": {
                                    "type": "boolean",
                                    "description": "Shuold use results from local cache if they are available. Othervise force to use latest one."
                                }
                            
                            }
                        }
                    }
                }),
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": "browse",
                        "description": "Browse a dynamic webpage",
                        "parameters": {
                            "type": "object",
                            "required": ["url"],
                            "properties": {
                                "url": {
                                    "type": "string",
                                    "description": "The URL to browse"
                                }
                            }
                        }
                    }
                }),
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": "scrape",
                        "description": "Scrape content from a static webpage",
                        "parameters": {
                            "type": "object",
                            "required": ["url"],
                            "properties": {
                                "url": {
                                    "type": "string",
                                    "description": "The URL to scrape"
                                }
                            }
                        }
                    }
                })
            ]),
        };


        dbg!(&request);

        let response = self.base.client
            .post(format!("{}/api/chat", self.base.config.ollama.base_url))
            .json(&request)
            .send()
            .await?;

        dbg!(&response);
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(KowalskiError::Server(error_text));
        }

        Ok(response)
    }

    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<Message>, KowalskiError> {
        self.base.process_stream_response(conversation_id, chunk).await
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        self.base.add_message(conversation_id, role, content).await
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn description(&self) -> &str {
        &self.base.description
    }
}

impl UnifiedAgent {
    /// Determines if the input requires tool usage
    fn should_use_tools(&self, content: &str) -> bool {
        content.starts_with("http") || 
        content.contains("search:") || 
        content.contains("browse:") || 
        content.contains("scrape:")
    }

    /// Execute a specific tool directly
    pub async fn execute_tool(&self, tool_type: TaskType, query: &str) -> Result<String, KowalskiError> {
        let tool_input = crate::tools::ToolInput::new(query.to_string());
        let tool_output = self.tool_chain.execute(tool_input).await?;
        Ok(tool_output.content)
    }

    /// Configure tool-specific options
    pub fn configure_tool(&mut self, tool_type: TaskType, options: HashMap<String, String>) {
        // TODO: Implement tool configuration
        debug!("Configuring tool {:?} with options: {:?}", tool_type, options);
    }
}