use crate::config::CodeAgentConfig;
use async_trait::async_trait;
use kowalski_agent_template::TemplateAgent;
use kowalski_agent_template::templates::general::GeneralTemplate;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::Conversation;
use kowalski_core::error::KowalskiError;
use kowalski_core::role::Role;
use kowalski_core::tools::{Tool, ToolOutput};
use kowalski_tools::code::{JavaAnalysisTool, PythonAnalysisTool, RustAnalysisTool};
use reqwest::Response;

/// CodeAgent: A specialized agent for code analysis and development tasks
/// This agent is built on top of the TemplateAgent and provides code-specific functionality
#[allow(dead_code)]
pub struct CodeAgent {
    agent: TemplateAgent,
    config: CodeAgentConfig,
}

impl CodeAgent {
    /// Creates a new CodeAgent with the specified configuration
    pub async fn new(_config: Config) -> Result<Self, KowalskiError> {
        // TODO: Convert Config to CodeAgentConfig if needed
        let code_config = CodeAgentConfig::default();

        // Create language-specific analysis tools
        let java_tool = JavaAnalysisTool::new();
        let python_tool = PythonAnalysisTool::new();
        let rust_tool = RustAnalysisTool::new();

        let tools: Vec<Box<dyn Tool + Send + Sync>> = vec![
            Box::new(java_tool),
            Box::new(python_tool),
            Box::new(rust_tool),
        ];

        let system_prompt = r#"You are a code analysis assistant. You can analyze Java, Python, and Rust code.

AVAILABLE TOOLS:
1. java_analysis - Analyzes a snippet of Java code.
   - parameters: { "content": "java code snippet" }
2. python_analysis - Analyzes a snippet of Python code.
   - parameters: { "content": "python code snippet" }
3. rust_analysis - Analyzes a snippet of Rust code.
   - parameters: { "content": "rust code snippet" }

TOOL USAGE INSTRUCTIONS:
- Use the appropriate tool for the language you want to analyze.
- Provide the code to analyze in the "content" parameter.

RESPONSE FORMAT:
When you need to use a tool, respond with JSON in this exact format:
{
  "name": "java_analysis",
  "parameters": {
    "content": "public class HelloWorld { ... }"
  },
  "reasoning": "I need to analyze this Java code."
}

When you have a final answer, respond normally without JSON formatting."#
            .to_string();
        let system_prompt_clone = system_prompt.clone();
        let builder = GeneralTemplate::create_agent(
            tools,
            Some(system_prompt),
            Some(0.7),
        )
        .await
        .map_err(|e| KowalskiError::Configuration(e.to_string()))?;
        let mut agent = builder.build().await?;
        // Ensure the system prompt is set on the base agent
        agent.base_mut().set_system_prompt(&system_prompt_clone);
        Ok(Self {
            agent,
            config: code_config,
        })
    }

    pub async fn list_tools(&self) -> Vec<(String, String)> {
        self.agent.list_tools().await
    }
}

#[async_trait]
impl Agent for CodeAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        CodeAgent::new(config).await
    }

    fn start_conversation(&mut self, model: &str) -> String {
        let system_prompt = {
            let base = self.agent.base();
            base.system_prompt.as_deref().unwrap_or("You are a helpful assistant.").to_string()
        };
        let conv_id = self.agent.base_mut().start_conversation(model);
        if let Some(conversation) = self.agent.base_mut().conversations.get_mut(&conv_id) {
            conversation.add_message("system", &system_prompt);
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
    ) -> Result<Response, KowalskiError> {
        self.agent
            .base_mut()
            .chat_with_history(conversation_id, content, role)
            .await
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

    async fn execute_tool(
        &mut self,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) -> Result<ToolOutput, KowalskiError> {
        self.agent.execute_tool(tool_name, tool_input).await
    }

    fn name(&self) -> &str {
        "Code Agent"
    }

    fn description(&self) -> &str {
        "A specialized agent for code analysis tasks."
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kowalski_core::config::Config;

    #[tokio::test]
    async fn test_code_agent_creation() {
        let config = Config::default();
        let agent = CodeAgent::new(config).await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_code_agent_conversation() {
        let config = Config::default();
        let mut agent = CodeAgent::new(config).await.unwrap();
        let conv_id = agent.start_conversation("test-model");
        assert!(!conv_id.is_empty());
    }
}
