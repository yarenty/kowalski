/// Academic Agent: Because reading papers wasn't complicated enough.
/// "Academia is like a maze where everyone pretends to know the way out." - A Confused Scholar

use async_trait::async_trait;
use reqwest::Response;
use crate::config::Config;
use crate::conversation::Conversation;
use crate::role::Role;
use super::{Agent, AgentError, BaseAgent};
use super::types::{ChatRequest, StreamResponse};
use crate::utils::{PdfReader, PaperCleaner};

/// AcademicAgent: Your personal research assistant with a PhD in sarcasm.
pub struct AcademicAgent {
    base: BaseAgent,
    pdf_reader: PdfReader,
    paper_cleaner: PaperCleaner,
}

#[async_trait]
impl Agent for AcademicAgent {
    fn new(config: Config) -> Result<Self, AgentError> {
        let base = BaseAgent::new(
            config,
            "Academic Agent",
            "A sophisticated paper processor that pretends to understand research better than you do",
        )?;

        Ok(Self {
            base,
            pdf_reader: PdfReader::new(),
            paper_cleaner: PaperCleaner::new(),
        })
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
    ) -> Result<Response, AgentError> {
        let conversation = self.base.conversations.get_mut(conversation_id)
            .ok_or_else(|| AgentError::ServerError("Conversation not found".to_string()))?;

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

        // Process content if it's a PDF file
        let processed_content = if content.ends_with(".pdf") {
            let pdf_content = self.pdf_reader.read_pdf(content)?;
            self.paper_cleaner.clean(&pdf_content)?
        } else {
            content.to_string()
        };

        conversation.add_message("user", &processed_content);

        let request = ChatRequest {
            model: conversation.model.clone(),
            messages: conversation.messages.iter()
                .map(|m| super::types::Message::from(m.clone()))
                .collect(),
            stream: true,
            temperature: self.base.config.chat.temperature.unwrap_or(0.7),
            max_tokens: self.base.config.chat.max_tokens.unwrap_or(2048) as usize,
        };

        let response = self.base.client
            .post(format!("{}/api/chat", self.base.config.ollama.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::ServerError(error_text));
        }

        Ok(response)
    }

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
        self.base.add_message(conversation_id, role, content).await
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn description(&self) -> &str {
        &self.base.description
    }
}

#[allow(dead_code)]
impl AcademicAgent {
    /// Processes an academic paper, because apparently reading it normally is too mainstream.
    pub async fn process_paper(&self, path: &str) -> Result<String, AgentError> {
        let content = self.pdf_reader.read_pdf(path)?;
        let cleaned = self.paper_cleaner.clean(&content)?;
        Ok(cleaned)
    }

    /// Extracts paper metadata, because titles and abstracts are too obvious.
    pub async fn extract_metadata(&self, path: &str) -> Result<PaperMetadata, AgentError> {
        let content = self.pdf_reader.read_pdf(path)?;
        
        // TODO:Add your metadata extraction logic here
        Ok(PaperMetadata {
            title: String::new(),
            authors: Vec::new(),
            abstract_text: content,
            keywords: Vec::new(),
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PaperMetadata {
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub keywords: Vec<String>,
} 