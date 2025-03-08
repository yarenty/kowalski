/// Error types for tools, because even tools make mistakes.
/// "Errors in tools are like bugs in a garden - they're everywhere but we pretend not to see them." - A Tool Developer

use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub enum ToolError {
    RequestError(reqwest::Error),
    JsonError(serde_json::Error),
    NoOutput,
    CacheError(String),
    RateLimitError(String),
    ScrapingError(String),
    BrowserError(String),
    SearchError(String),
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolError::RequestError(e) => write!(f, "Request error: {}", e),
            ToolError::JsonError(e) => write!(f, "JSON error: {}", e),
            ToolError::NoOutput => write!(f, "No output produced"),
            ToolError::CacheError(e) => write!(f, "Cache error: {}", e),
            ToolError::RateLimitError(e) => write!(f, "Rate limit error: {}", e),
            ToolError::ScrapingError(e) => write!(f, "Scraping error: {}", e),
            ToolError::BrowserError(e) => write!(f, "Browser error: {}", e),
            ToolError::SearchError(e) => write!(f, "Search error: {}", e),
        }
    }
}

impl Error for ToolError {} 