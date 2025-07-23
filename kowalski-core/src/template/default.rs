use crate::template::builder::AgentBuilder;
use crate::tools::Tool;

pub struct DefaultTemplate;

impl DefaultTemplate {
    /// Creates a new general-purpose agent with customizable tools
    pub async fn create_agent(
        tools: Vec<Box<dyn Tool + Send + Sync>>,
        system_prompt: Option<String>,
        temperature: Option<f32>,
    ) -> Result<AgentBuilder, Box<dyn std::error::Error>> {
        let default_prompt = "You are a versatile AI assistant that can help with various tasks.";
        let prompt = system_prompt.unwrap_or_else(|| default_prompt.to_string());
        let temp = temperature.unwrap_or(0.7);

        let builder = AgentBuilder::new()
            .await
            .with_system_prompt(&prompt)
            .with_tools(tools)
            .with_temperature(temp);

        Ok(builder)
    }

    /// Creates a default general-purpose agent with basic tools
    pub async fn create_default_agent() -> Result<AgentBuilder, Box<dyn std::error::Error>> {
        Self::create_agent(Vec::new(), None, None).await
    }
}
