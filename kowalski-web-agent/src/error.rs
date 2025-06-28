use kowalski_core::error::KowalskiError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WebAgentError {
    #[error("Core error: {0}")]
    Core(#[from] KowalskiError),

    #[error("Web scraping error: {0}")]
    Scraping(String),

    #[error("Web browser error: {0}")]
    Browser(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Content processing error: {0}")]
    ContentProcessing(String),
}

impl From<reqwest::Error> for WebAgentError {
    fn from(err: reqwest::Error) -> Self {
        WebAgentError::Network(err.to_string())
    }
}

impl From<url::ParseError> for WebAgentError {
    fn from(err: url::ParseError) -> Self {
        WebAgentError::InvalidUrl(err.to_string())
    }
}
