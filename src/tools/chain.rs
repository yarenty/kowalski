use super::{TaskType, Tool, ToolError, ToolInput, ToolOutput, ToolType};
use async_trait::async_trait;
use log::{debug, error, info};
use std::collections::HashMap;

/// ToolChain: Manages and executes a chain of tools
pub struct ToolChain {
    tools: HashMap<TaskType, Vec<ToolType>>,
}

impl ToolChain {
    /// Creates a new ToolChain
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Adds a tool to the chain
    pub fn add_tool(&mut self, tool: ToolType, task_types: Vec<TaskType>) {
        for task_type in task_types {
            self.tools
                .entry(task_type)
                .or_insert_with(Vec::new)
                .push(tool.clone());
        }
    }

    /// Removes a tool from the chain
    pub fn remove_tool(&mut self, task_type: TaskType, tool: &ToolType) {
        if let Some(tools) = self.tools.get_mut(&task_type) {
            tools.retain(|t| t != tool);
        }
    }

    /// Executes the appropriate tool for the given input
    pub async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        debug!("Executing tool chain for input: {}", input.query);

        // Determine the task type from the input
        let task_type = self.determine_task_type(&input.query)?;
        
        // Get the appropriate tools for this task
        let tools = self.tools.get(&task_type).ok_or_else(|| {
            ToolError::NoToolAvailable(format!("No tools available for task type: {:?}", task_type))
        })?;

        // Try each tool until one succeeds
        for tool in tools {
            match self.execute_tool(tool, &input).await {
                Ok(output) => {
                    info!("Tool execution successful: {:?}", tool);
                    return Ok(output);
                }
                Err(e) => {
                    error!("Tool execution failed: {:?} - {}", tool, e);
                    continue;
                }
            }
        }

        Err(ToolError::ExecutionFailed("All tools failed to execute".to_string()))
    }

    /// Determines the task type from the input query
    fn determine_task_type(&self, query: &str) -> Result<TaskType, ToolError> {
        if query.starts_with("http") {
            Ok(TaskType::BrowseDynamic)
        } else if query.contains("search:") {
            Ok(TaskType::Search)
        } else if query.contains("scrape:") {
            Ok(TaskType::ScrapStatic)
        } else {
            Err(ToolError::InvalidInput("Could not determine task type".to_string()))
        }
    }

    /// Executes a specific tool
    async fn execute_tool(&self, tool: &ToolType, input: &ToolInput) -> Result<ToolOutput, ToolError> {
        match tool {
            ToolType::Search(tool) => tool.execute(input.clone()).await,
            ToolType::Browser(tool) => tool.execute(input.clone()).await,
            ToolType::Scraper(tool) => tool.execute(input.clone()).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::SearchTool;
    use crate::tools::SearchProvider;

    #[test]
    fn test_tool_chain_creation() {
        let chain = ToolChain::new();
        assert!(chain.tools.is_empty());
    }

    #[test]
    fn test_add_tool() {
        let mut chain = ToolChain::new();
        let search_tool = ToolType::Search(SearchTool::new(SearchProvider::DuckDuckGo, String::new()));
        chain.add_tool(search_tool.clone(), vec![TaskType::Search]);
        
        assert!(chain.tools.contains_key(&TaskType::Search));
        assert_eq!(chain.tools.get(&TaskType::Search).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_execute_tool() {
        let mut chain = ToolChain::new();
        let search_tool = ToolType::Search(SearchTool::new(SearchProvider::DuckDuckGo, String::new()));
        chain.add_tool(search_tool, vec![TaskType::Search]);

        let input = ToolInput {
            query: "search: test query".to_string(),
            parameters: HashMap::new(),
        };

        let result = chain.execute(input).await;
        assert!(result.is_ok());
    }
} 