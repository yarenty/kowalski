use crate::config::CodeAgentConfig;
use async_trait::async_trait;
use kowalski_agent_template::TemplateAgent;
use kowalski_agent_template::templates::general::GeneralTemplate;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::Conversation;
use kowalski_core::error::KowalskiError;
use kowalski_core::role::Role;
use kowalski_core::tools::{Tool, ToolInput};
use kowalski_tools::code::{JavaAnalysisTool, PythonAnalysisTool, RustAnalysisTool};
use serde::{Deserialize, Serialize};

/// CodeAgent: A specialized agent for code analysis and development tasks
/// This agent is built on top of the TemplateAgent and provides code-specific functionality
pub struct CodeAgent {
    agent: TemplateAgent,
    config: CodeAgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysisResult {
    pub language: String,
    pub metrics: serde_json::Value,
    pub suggestions: Vec<String>,
    pub issues: Vec<String>,
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

        let builder = GeneralTemplate::create_agent(
            tools,
            Some("You are a code analysis and development assistant specialized in analyzing, refactoring, and documenting code.".to_string()),
            Some(0.7),
        )
        .await
        .map_err(|e| KowalskiError::Configuration(e.to_string()))?;
        let agent = builder.build().await?;

        Ok(Self {
            agent,
            config: code_config,
        })
    }

    /// Analyzes Java code
    pub async fn analyze_java(&self, code: &str) -> Result<CodeAnalysisResult, KowalskiError> {
        let mut tools = self.agent.tool_chain.write().await;
        let tool = tools.iter_mut().find(|t| t.name() == "java_analysis");
        let tool = match tool {
            Some(t) => t,
            None => {
                return Err(KowalskiError::ToolExecution(
                    "java_analysis tool not found".to_string(),
                ));
            }
        };
        let input = ToolInput::new(
            "analyze_java".to_string(),
            code.to_string(),
            serde_json::json!({}),
        );
        let output = tool.execute(input).await?;

        let result = output.result;
        Ok(CodeAnalysisResult {
            language: "java".to_string(),
            metrics: result["metrics"].clone(),
            suggestions: result["suggestions"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            issues: result["syntax_errors"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
        })
    }

    /// Analyzes Python code
    pub async fn analyze_python(&self, code: &str) -> Result<CodeAnalysisResult, KowalskiError> {
        let mut tools = self.agent.tool_chain.write().await;
        let tool = tools.iter_mut().find(|t| t.name() == "python_analysis");
        let tool = match tool {
            Some(t) => t,
            None => {
                return Err(KowalskiError::ToolExecution(
                    "python_analysis tool not found".to_string(),
                ));
            }
        };
        let input = ToolInput::new(
            "analyze_python".to_string(),
            code.to_string(),
            serde_json::json!({}),
        );
        let output = tool.execute(input).await?;

        let result = output.result;
        Ok(CodeAnalysisResult {
            language: "python".to_string(),
            metrics: result["metrics"].clone(),
            suggestions: result["suggestions"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            issues: result["pep8_issues"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
        })
    }

    /// Analyzes Rust code
    pub async fn analyze_rust(&self, code: &str) -> Result<CodeAnalysisResult, KowalskiError> {
        let mut tools = self.agent.tool_chain.write().await;
        let tool = tools.iter_mut().find(|t| t.name() == "rust_analysis");
        let tool = match tool {
            Some(t) => t,
            None => {
                return Err(KowalskiError::ToolExecution(
                    "rust_analysis tool not found".to_string(),
                ));
            }
        };
        let input = ToolInput::new(
            "analyze_rust".to_string(),
            code.to_string(),
            serde_json::json!({}),
        );
        let output = tool.execute(input).await?;

        let result = output.result;
        Ok(CodeAnalysisResult {
            language: "rust".to_string(),
            metrics: result["metrics"].clone(),
            suggestions: result["suggestions"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            issues: result["rust_issues"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
        })
    }
}

#[async_trait]
impl Agent for CodeAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        CodeAgent::new(config).await
    }

    fn start_conversation(&mut self, model: &str) -> String {
        self.agent.base_mut().start_conversation(model)
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
