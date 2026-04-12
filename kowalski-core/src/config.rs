use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core configuration for the Kowalski system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Ollama configuration
    pub ollama: OllamaConfig,
    /// Chat configuration
    pub chat: ChatConfig,
    /// Qdrant configuration
    pub qdrant: QdrantConfig,
    /// Memory configuration
    pub memory: MemoryConfig,
    /// Maximum number of memories to retrieve from working memory
    pub working_memory_retrieval_limit: usize,
    /// Maximum number of memories to retrieve from episodic memory
    pub episodic_memory_retrieval_limit: usize,
    /// Maximum number of memories to retrieve from semantic memory
    pub semantic_memory_retrieval_limit: usize,
    /// LLM configuration (new)
    #[serde(default)]
    pub llm: LLMConfig,
    /// MCP configuration
    #[serde(default)]
    pub mcp: McpConfig,
    /// Additional configurations from other agents
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

/// Configuration for generic LLM settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// The provider to use: "ollama", "openai", etc.
    pub provider: String,
    /// OpenAI API key (if using openai provider)
    pub openai_api_key: Option<String>,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
        }
    }
}

/// Configuration for Ollama integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// The host where Ollama is running
    pub host: String,
    /// The port where Ollama is running
    pub port: u16,
    /// The model to use
    pub model: String,
    /// Additional Ollama-specific settings
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 11434,
            model: "llama3.2".to_string(), //llama3.2 //deepseek-r1:1.5b
            additional: HashMap::new(),
        }
    }
}

/// Configuration for chat functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    /// Maximum number of messages to keep in history
    pub max_history: usize,
    /// Whether to enable streaming responses
    pub enable_streaming: bool,
    /// Temperature for response generation (0.0 to 1.0)
    pub temperature: f32,
    /// Maximum number of tokens in generated responses
    pub max_tokens: u32,
    /// Additional chat-specific settings
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            max_history: 100,
            enable_streaming: true,
            temperature: 0.7,
            max_tokens: 2048,
            additional: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    pub http_url: String,
    pub grpc_url: String,
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            http_url: "http://localhost:6333".to_string(),
            grpc_url: "http://localhost:6334".to_string(),
            additional: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub episodic_path: String,
    /// Optional SQL store: prefer **`sqlite:…`** (single file, no server) for the simple path; use **`postgres://…`** when you need Postgres + pgvector at scale.
    /// If unset, no SQL migrations run and existing RocksDB/Qdrant memory paths apply unchanged.
    #[serde(default)]
    pub database_url: Option<String>,
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            episodic_path: "../target/episodic_db".to_string(), //just for testing!
            database_url: None,
            additional: HashMap::new(),
        }
    }
}

/// Trait for extending configuration with additional settings
pub trait ConfigExt {
    /// Get a reference to the core configuration
    fn core(&self) -> &Config;

    /// Get a mutable reference to the core configuration
    fn core_mut(&mut self) -> &mut Config;

    /// Get additional configuration value by key
    fn get_additional<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.core()
            .additional
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set additional configuration value
    fn set_additional<T: serde::Serialize>(&mut self, key: &str, value: T) {
        if let Ok(json) = serde_json::to_value(value) {
            self.core_mut().additional.insert(key.to_string(), json);
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ollama: OllamaConfig::default(),
            llm: LLMConfig::default(),
            mcp: McpConfig::default(),
            chat: ChatConfig::default(),
            qdrant: QdrantConfig::default(),
            memory: MemoryConfig::default(),
            working_memory_retrieval_limit: 3,
            episodic_memory_retrieval_limit: 3,
            semantic_memory_retrieval_limit: 3,
            additional: HashMap::new(),
        }
    }
}

/// Configuration for MCP servers
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    /// Base URL of the MCP server (JSON-RPC endpoint or HTTP transport root)
    pub url: String,
    /// Preferred transport, defaults to SSE as per spec
    #[serde(default)]
    pub transport: McpTransport,
    /// Optional static headers (e.g., auth tokens)
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    Sse,
    Http,
}

impl Default for McpTransport {
    fn default() -> Self {
        McpTransport::Sse
    }
}
