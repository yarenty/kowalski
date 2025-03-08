pub struct SearchTool {
    provider: SearchProvider,
    api_key: String,
}

impl SearchTool {
    pub fn new(provider: SearchProvider, api_key: String) -> Self {
        Self { provider, api_key }
    }

    pub async fn query(&self, query: &str) -> Result<Vec<SearchResult>, ToolError> {
        match self.provider {
            SearchProvider::Google => self.google_search(query).await,
            SearchProvider::DuckDuckGo => self.duckduckgo_search(query).await,
        }
    }
}

#[async_trait]
impl Tool for SearchTool {
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let results = self.query(&input.query).await?;
        let content = serde_json::to_string(&results)?;
        
        Ok(ToolOutput {
            content,
            metadata: HashMap::new(),
            source: Some(format!("search:{}", self.provider)),
        })
    }
}