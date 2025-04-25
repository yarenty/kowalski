use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum KowalskiError {
    // Generic errors
    Request(reqwest::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
    Config(String),
    Serialization(String),

    // Agent specific errors
    Server(String),
    ConversationNotFound(String),

    // Tool specific errors
    NoOutput,
    Cache(String),
    RateLimit(String),
    Scraping(String),
    Browser(String),
    Search(String),
    NoSuitableTool(String),

    InvalidPath(String),
    PdfError(String),

    // Paper cleaner errors
    InvalidInput(String),
}

impl fmt::Display for KowalskiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Generic errors
            KowalskiError::Request(e) => write!(f, "Request error: {}", e),
            KowalskiError::Json(e) => write!(f, "JSON error: {}", e),
            KowalskiError::Io(e) => write!(f, "IO error: {}", e),
            KowalskiError::Config(e) => write!(f, "Config error: {}", e),
            KowalskiError::Serialization(e) => write!(f, "Serialization error: {}", e),

            // Agent specific errors
            KowalskiError::Server(e) => write!(f, "Server error: {}", e),
            KowalskiError::ConversationNotFound(e) => write!(f, "Conversation not found: {}", e),

            // Tool specific errors
            KowalskiError::NoOutput => write!(f, "No output produced"),
            KowalskiError::Cache(e) => write!(f, "Cache error: {}", e),
            KowalskiError::RateLimit(e) => write!(f, "Rate limit error: {}", e),
            KowalskiError::Scraping(e) => write!(f, "Scraping error: {}", e),
            KowalskiError::Browser(e) => write!(f, "Browser error: {}", e),
            KowalskiError::Search(e) => write!(f, "Search error: {}", e),
            KowalskiError::NoSuitableTool(e) => write!(f, "No suitable tool found: {}", e),

            KowalskiError::InvalidPath(e) => write!(f, "Invalid path: {}", e),
            KowalskiError::PdfError(e) => write!(f, "PDF error: {}", e),

            // Paper cleaner errors
            KowalskiError::InvalidInput(e) => write!(f, "Invalid input: {}", e),
        }
    }
}

impl Error for KowalskiError {}

// Implement conversions from common error types
impl From<reqwest::Error> for KowalskiError {
    fn from(err: reqwest::Error) -> Self {
        KowalskiError::Request(err)
    }
}

impl From<serde_json::Error> for KowalskiError {
    fn from(err: serde_json::Error) -> Self {
        KowalskiError::Json(err)
    }
}

impl From<std::io::Error> for KowalskiError {
    fn from(err: std::io::Error) -> Self {
        KowalskiError::Io(err)
    }
}

impl From<config::ConfigError> for KowalskiError {
    fn from(err: config::ConfigError) -> Self {
        KowalskiError::Config(err.to_string())
    }
}
