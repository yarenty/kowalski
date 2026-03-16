use thiserror::Error;

#[derive(Error, Debug)]
pub enum KowalskiCliError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Agent error: {0}")]
    Agent(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
}
