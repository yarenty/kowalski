use thiserror::Error;

#[derive(Error, Debug)]
pub enum KowalskiError {
    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Content processing error: {0}")]
    ContentProcessing(String),

    #[error("Invalid input: {0}")]
    ToolInvalidInput(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Task error: {0}")]
    Task(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Template agent error: {0}")]
    TemplateAgent(String),

    #[error("Web agent error: {0}")]
    WebAgent(String),

    #[error("Academic agent error: {0}")]
    AcademicAgent(String),

    #[error("Tool chain error: {0}")]
    ToolChain(String),

    #[error("Task handler error: {0}")]
    TaskHandler(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Rate limit error: {0}")]
    RateLimit(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Resource error: {0}")]
    Resource(String),

    #[error("State error: {0}")]
    State(String),

    #[error("Initialization error: {0}")]
    Initialization(String),

    #[error("Shutdown error: {0}")]
    Shutdown(String),

    #[error("Recovery error: {0}")]
    Recovery(String),

    #[error("Cleanup error: {0}")]
    Cleanup(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Conversation not found: {0}")]
    ConversationNotFound(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Network error: {0}")]
    ToolNetwork(String),

    #[error("Config error: {0}")]
    ToolConfig(String),
}

impl From<String> for KowalskiError {
    fn from(err: String) -> Self {
        KowalskiError::Agent(err)
    }
}

impl From<&str> for KowalskiError {
    fn from(err: &str) -> Self {
        KowalskiError::Agent(err.to_string())
    }
}
