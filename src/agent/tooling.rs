/// Tooling Agent: Because browsing the web manually is so last century.
/// "Web scraping is like archaeology - you dig through layers of HTML hoping to find treasure." - A Digital Archaeologist

use async_trait::async_trait;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::config::Config;
use crate::conversation::Conversation;
use crate::role::Role;
use super::{Agent, AgentError, BaseAgent};
use crate::tools::{ ToolChain,  SearchTool,  ToolCache,  ToolInput, ToolType};
use crate::tools::search::SearchProvider;
use log::{debug, info};
use crate::tools::TaskType;
use crate::tools::browser::WebBrowser;
use crate::tools::scraper::WebScraper;

/// ToolingAgent: Your personal web crawler with a sense of humor.
#[allow(dead_code)]
pub struct ToolingAgent {
    base: BaseAgent,
    chain: ToolChain,
    cache: ToolCache,
    search_limit: usize,
    follow_links: bool,
    max_depth: u32,
    code_language: Option<String>,
}

impl ToolingAgent {
    /// Creates a new tooling agent
    pub fn new(config: Config) -> Result<Self, AgentError> {
        let base = BaseAgent::new(
            config.clone(),
            "Tooling Agent",
            "A versatile agent that uses various tools to process information",
        )?;

        let mut chain = ToolChain::new();
        
        // Register WebBrowser for dynamic content
        chain.add_tool(
            ToolType::Browser(WebBrowser::new(config.clone())),
            vec![TaskType::BrowseDynamic]
        );
        
        // Register SearchTool for search queries
        chain.add_tool(
            ToolType::Search(SearchTool::new(
                SearchProvider::DuckDuckGo,
                config.search.api_key.clone().unwrap_or_default(),
            )),
            vec![TaskType::Search]
        );
        
        // Register WebScraper for static content
        chain.add_tool(
            ToolType::Scraper(WebScraper::new()),
            vec![TaskType::ScrapStatic]
        );

        let cache = ToolCache::new();

        Ok(Self {
            base,
            chain,
            cache,
            search_limit: 5,
            follow_links: false,
            max_depth: 1,
            code_language: None,
        })
    }

    /// Set the search result limit
    pub fn set_search_limit(&mut self, limit: usize) {
        self.search_limit = limit;
    }

    /// Set scraping options
    pub fn set_scrape_options(&mut self, follow_links: bool, max_depth: u32) {
        self.follow_links = follow_links;
        self.max_depth = max_depth;
    }

    /// Set the code language for analysis
    pub fn set_code_language(&mut self, language: &str) {
        self.code_language = Some(language.to_string());
    }
}

#[async_trait]
impl Agent for ToolingAgent {
    fn new(config: Config) -> Result<Self, AgentError> {
        Self::new(config)
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
        // Process content through tool chain if it looks like a web request
        let processed_content = if content.starts_with("http") || content.contains("search:") {
            let tool_input = ToolInput::new(content.to_string());
            let tool_output = self.chain.execute(tool_input).await?;
            tool_output.content
        } else {
            content.to_string()
        };

        self.base.chat_with_history(conversation_id, &processed_content, role).await
    }

    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<String>, AgentError> {
        self.base.process_stream_response(conversation_id, chunk).await
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        self.base.add_message(conversation_id, role, content).await
    }

    fn name(&self) -> &str {
        self.base.name()
    }

    fn description(&self) -> &str {
        self.base.description()
    }
}

impl ToolingAgent {
    /// Searches the web, because Google is too mainstream.
    pub async fn search(&mut self, query: &str) -> Result<Vec<SearchResult>, AgentError> {
        let tool_input = ToolInput::new(query.to_string());
        debug!("Searching for: {}", query);
        let output = self.chain.execute(tool_input).await?;
        debug!("Raw HTML output received");

        let document = scraper::Html::parse_document(&output.content);
        debug!("Parsed HTML document");
        info!("HTML document: {:.100}", &document.html());
        document.select(&scraper::Selector::parse("link").unwrap_or_else(|_| {
            scraper::Selector::parse("a").unwrap_or_else(|_| {
                scraper::Selector::parse("a").expect("Failed to create link selector")
            })
        })).for_each(|el| {
            debug!("Link: {:?}", el.value().attr("href"));
        });


        let mut results = Vec::new();

        // DuckDuckGo specific selectors
        let result_selector = scraper::Selector::parse(".result__body").unwrap_or_else(|_| {
            scraper::Selector::parse(".nrn-react-div").unwrap_or_else(|_| {
                scraper::Selector::parse(".web-result").expect("Failed to create result selector")
            })
        });

        let title_selector = scraper::Selector::parse(".result__title, .result__a").expect("Failed to create title selector");
        let url_selector = scraper::Selector::parse(".result__url").expect("Failed to create url selector");
        let snippet_selector = scraper::Selector::parse(".result__snippet").expect("Failed to create snippet selector");

        for element in document.select(&result_selector) {
            let title = element.select(&title_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let url = element.select(&url_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_else(|| {
                    element.select(&title_selector)
                        .next()
                        .and_then(|el| el.value().attr("href"))
                        .unwrap_or_default()
                        .to_string()
                });

            let snippet = element.select(&snippet_selector)
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            if !title.is_empty() {
                results.push(SearchResult {
                    title: title.trim().to_string(),
                    url: url.trim().to_string(),
                    snippet: snippet.trim().to_string(),
                });
            }
        }

        // If no results found, try alternative selectors
        if results.is_empty() {
            info!("No results found with primary selectors, trying alternatives");
            let link_selector = scraper::Selector::parse("link").expect("Failed to create link selector");
            let links: Vec<_> = document.select(&link_selector).collect();
            info!("Found {} links", links.len());
            
            for link in links {
                if let (Some(href), Some(text)) = (link.value().attr("href"), Some(link.text().collect::<String>())) {
                    info!("Link: {:?}", href);
                    if (href.starts_with("http") || href.starts_with("https")) && !text.trim().is_empty() {
                        results.push(SearchResult {
                            title: text.trim().to_string(),
                            url: href.to_string(),
                            snippet: String::new(),
                        });
                    }
                }
            }
        }

        debug!("Parsed {} search results", results.len());
        Ok(results)
    }

    /// Fetches and processes a webpage, because raw HTML is for machines.
    pub async fn fetch_page(&mut self, url: &str) -> Result<ProcessedPage, AgentError> {
        let tool_input = ToolInput::new(url.to_string());
        let output = self.chain.execute(tool_input).await?;
        
        let document = scraper::Html::parse_document(&output.content);
        
        // Extract title from meta tags or title tag
        let title = document
            .select(&scraper::Selector::parse("title").expect("Failed to create title selector"))
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default();

        info!("[{}:{}] Title extracted: {}", file!(), line!(), &title);

        // Extract main content, prioritizing main content areas
        let content_selector = scraper::Selector::parse("article, main, .content, .main-content, body").expect("Failed to create content selector");
        let content = document
            .select(&content_selector)
            .map(|el| el.text().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        debug!("[{}:{}] Content length: {} bytes", file!(), line!(), content.len());

        // Extract metadata from meta tags
        let meta_selector = scraper::Selector::parse("meta[name][content], meta[property][content]").expect("Failed to create meta selector");
        let mut metadata = HashMap::new();
        for meta in document.select(&meta_selector) {
            if let (Some(name), Some(content)) = (
                meta.value().attr("name").or(meta.value().attr("property")),
                meta.value().attr("content")
            ) {
                metadata.insert(name.to_string(), content.to_string());
            }
        }

        info!("[{}:{}] Metadata entries: {}", file!(), line!(), metadata.len());
        debug!("[{}:{}] Content preview: {:.100}...", file!(), line!(), content);

        Ok(ProcessedPage {
            url: url.to_string(),
            title,
            content,
            metadata,
        })
    }

    /// Collects data from multiple pages, because one page is never enough.
    #[allow(dead_code)]
    pub async fn collect_data(&mut self, urls: Vec<String>) -> Result<Vec<ProcessedPage>, AgentError> {
        let mut results = Vec::new();
        for url in urls {
            if let Ok(page) = self.fetch_page(&url).await {
                results.push(page);
            }
        }
        Ok(results)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedPage {
    pub url: String,
    pub title: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
} 