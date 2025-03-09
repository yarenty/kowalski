use crate::agent::{Agent, AgentError, BaseAgent};
use crate::config::Config;
use super::types::StreamResponse;
use crate::role::Role;
use reqwest::Response;

/// GeneralAgent: A simple agent for basic chat interactions
/// 
/// This agent provides straightforward chat functionality without specialized features,
/// making it ideal for general conversation and simple Q&A tasks.
pub struct GeneralAgent {
    base: BaseAgent,
    base_system_prompt: String,
}

impl GeneralAgent {

    /// Sets a custom system prompt for the agent
    #[allow(dead_code)]
    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.base_system_prompt = prompt.to_string();
        self
    }
}

#[async_trait::async_trait]
impl Agent for GeneralAgent {
    fn new(config: Config) -> Result<Self, AgentError> {
        Ok(Self {
            base: BaseAgent::new(
                config,
                "GeneralAgent",
                "A general-purpose chat agent for basic interactions",
            )?,
            base_system_prompt: String::from(
                "You are a helpful AI assistant. Provide clear, concise, and accurate responses.",
            ),
        })
    }

    fn start_conversation(&mut self, model: &str) -> String {
        self.base.start_conversation(model)
    }

    fn get_conversation(&self, id: &str) -> Option<&crate::conversation::Conversation> {
        self.base.get_conversation(id)
    }

    fn list_conversations(&self) -> Vec<&crate::conversation::Conversation> {
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
    ) -> Result<Response, AgentError> {

        // Still could use some role-based system messages
        if let Some(role) = role {
            self.add_message(conversation_id,"system", role.get_prompt()).await;
            
            if let Some(audience) = role.get_audience() {
                self.add_message(conversation_id,"system", audience.get_prompt()).await;
            }
            if let Some(preset) = role.get_preset() {
               self.add_message(conversation_id,"system", preset.get_prompt()).await;
            }
        }

        // Add the user's message to the conversation
        self.add_message(conversation_id, "user", content).await;


        // Get the conversation
        let conversation = self.base.get_conversation(conversation_id)
            .ok_or_else(|| AgentError::ConversationNotFound(conversation_id.to_string()))?;


        // Prepare the chat request
        let chat_request = serde_json::json!({
            "model": conversation.model,
            "messages": [
                {
                    "role": "system",
                    "content": &self.base_system_prompt
                },
                {
                    "role": "user",
                    "content": content
                }
            ],
            "stream": self.base.config.chat.stream,
            "temperature": self.base.config.chat.temperature,
            "max_tokens": self.base.config.chat.max_tokens,
        });

        dbg!(&chat_request);
        // Send the request
        let response = self.base.client
            .post(format!("{}/api/chat", self.base.config.ollama.base_url))
            .json(&chat_request)
            .send()
            .await
            .map_err(AgentError::RequestError)?;

        dbg!(&response);

        Ok(response)
    }

    // async fn process_stream_response(
    //     &mut self,
    //     conversation_id: &str,
    //     chunk: &[u8],
    // ) -> Result<Option<String>, AgentError> {
    //     debug!("Processing stream response for conversation {}", conversation_id);
        
    //     // Parse the chunk as a JSON response
    //     if let Ok(text) = String::from_utf8(chunk.to_vec()) {
    //         if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
    //             if let Some(response) = json["response"].as_str() {
    //                 // Add the assistant's response to the conversation
    //                 self.add_message(conversation_id, "assistant", response).await;
    //                 return Ok(Some(response.to_string()));
    //             }
    //         }
    //     }
        
    //     Ok(None)
    // }

    async fn process_stream_response(
        &mut self,
        _conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<String>, AgentError> {
        let text = String::from_utf8(chunk.to_vec())
            .map_err(|e| AgentError::ServerError(format!("Invalid UTF-8: {}", e)))?;

        let stream_response: StreamResponse = serde_json::from_str(&text)
            .map_err(|e| AgentError::JsonError(e))?;

        if stream_response.done {
            return Ok(None);
        }

        Ok(Some(stream_response.message.content))
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        self.base.add_message(conversation_id, role, content).await;
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn description(&self) -> &str {
        &self.base.description
    }
} 