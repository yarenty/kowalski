pub struct ToolChain {
    tools: Vec<Box<dyn Tool>>,
    cache: ToolCache,
}

impl ToolChain {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            cache: ToolCache::new(),
        }
    }

    pub fn add<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    pub async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let mut current_input = input;
        let mut final_output = None;

        for tool in &self.tools {
            if let Some(cached) = self.cache.get(&current_input) {
                final_output = Some(cached);
                continue;
            }

            let output = tool.execute(current_input).await?;
            self.cache.set(&current_input, &output);
            
            current_input = ToolInput {
                query: output.content,
                parameters: output.metadata,
            };
            
            final_output = Some(output);
        }

        final_output.ok_or(ToolError::NoOutput)
    }
}