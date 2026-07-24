use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Single source of truth for how kowalski processes find the HTTP API server.
/// The server binds here by default (`kowalski --bind`); the CLI targets
/// `http://<DEFAULT_API_BIND>` unless overridden; `ui/vite.config.ts` proxies to the
/// same address (keep it in sync — TypeScript cannot import this constant).
pub const DEFAULT_API_BIND: &str = "127.0.0.1:3456";

/// Env var carrying the server base URL (e.g. `http://127.0.0.1:3456`). Read by the CLI
/// (`--api` flag wins over it); exported by the server to every worker it spawns.
pub const API_URL_ENV: &str = "KOWALSKI_API";

/// Env var carrying the API bearer token. Read by the CLI on every `/api/*` call;
/// exported by the server to every worker it spawns; overrides the server's token file.
pub const API_TOKEN_ENV: &str = "KOWALSKI_API_TOKEN";

/// Env var overriding the horde-run store database URL (e.g. `sqlite:/path/runs.sqlite`).
/// Without it the store creates `runs.sqlite` under the server state dir — zero config.
pub const RUN_DB_ENV: &str = "KOWALSKI_RUN_DB";

/// Core configuration for the Kowalski system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Ollama configuration
    pub ollama: OllamaConfig,
    /// Chat configuration
    pub chat: ChatConfig,
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
    /// The provider to use: `ollama` (local) or `openai` (Chat Completions API — OpenAI or compatible).
    pub provider: String,
    /// API key for `openai` provider (OpenAI, Groq, etc.). Many servers omit this; use `""` in TOML if needed.
    pub openai_api_key: Option<String>,
    /// Base URL for OpenAI-compatible Chat Completions (e.g. `https://api.openai.com/v1`, or
    /// `http://127.0.0.1:1234/v1` for LM Studio). If unset, the official OpenAI API base is used.
    #[serde(default)]
    pub openai_api_base: Option<String>,
    /// Model name for the LLM provider. When using `openai` provider, this is the model ID to use.
    /// For Ollama, this can be left empty to use `ollama.model` as fallback.
    #[serde(default)]
    pub model: Option<String>,
    /// Provider to use for embeddings: `llm` (same as chat) or `ollama` (use Ollama's embeddings API).
    /// Set to `"ollama"` if your OpenAI-compatible API doesn't support embeddings.
    #[serde(default = "default_embeddings_provider")]
    pub embeddings_provider: String,
}

fn default_embeddings_provider() -> String {
    "llm".to_string()
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            openai_api_base: None,
            model: None,
            embeddings_provider: "llm".to_string(),
        }
    }
}

/// Configuration for Ollama integration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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
#[serde(default)]
pub struct ChatConfig {
    /// Maximum number of messages to keep in history
    pub max_history: usize,
    /// Whether to enable streaming responses (`stream` is accepted as a TOML field alias)
    #[serde(alias = "stream")]
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

fn default_embedding_vector_dimensions() -> usize {
    768
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    /// **Default Tier-2 episodic store:** embedded **SQLite** file under this path (`episodic.sqlite` in the directory, or a path ending in `.sqlite`/`.db`). Used when [`Self::database_url`] is unset or does not request PostgreSQL.
    pub episodic_path: String,
    /// Optional: set to **`postgres://…`** / **`postgresql://…`** to use PostgreSQL for Tier 2 (`episodic_kv`) and Tier 3 semantic SQL (**requires** `kowalski-core` **`--features postgres`**). If omitted, Tier 2 stays on **SQLite** ([`Self::episodic_path`]) — the default.
    #[serde(default)]
    pub database_url: Option<String>,
    /// Embedding width for **PostgreSQL** `semantic_memory.embedding` (`vector(N)`). Must match your embedder (e.g. **768** for Ollama `nomic-embed-text`) and the dimension in `migrations/postgres/003_semantic_memory.sql` (this crate).
    #[serde(default = "default_embedding_vector_dimensions")]
    pub embedding_vector_dimensions: usize,
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            episodic_path: "../target/episodic_db".to_string(), //just for testing!
            database_url: None,
            embedding_vector_dimensions: default_embedding_vector_dimensions(),
            additional: HashMap::new(),
        }
    }
}

/// Returns true when [`MemoryConfig::database_url`] points at PostgreSQL (episodic + semantic SQL backends).
pub fn memory_uses_postgres(memory: &MemoryConfig) -> bool {
    memory
        .database_url
        .as_ref()
        .is_some_and(|u| u.starts_with("postgres://") || u.starts_with("postgresql://"))
}

#[cfg(test)]
mod postgres_flag_tests {
    use super::{MemoryConfig, memory_uses_postgres};

    #[test]
    fn memory_uses_postgres_detects_url() {
        let mut m = MemoryConfig::default();
        assert!(!memory_uses_postgres(&m));
        m.database_url = Some("postgres://localhost/db".to_string());
        assert!(memory_uses_postgres(&m));
        m.database_url = Some("postgresql://localhost/db".to_string());
        assert!(memory_uses_postgres(&m));
    }
}

/// Build-time `postgres` feature was not enabled while config requests a PostgreSQL URL.
pub fn postgres_feature_required_error() -> crate::error::KowalskiError {
    crate::error::KowalskiError::Configuration(
        "PostgreSQL support requires building with `--features postgres` (e.g. `cargo build -p kowalski-core --features postgres` or `cargo build -p kowalski-cli --features postgres`).".to_string(),
    )
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
    /// Base URL for HTTP/SSE; ignored for `stdio` (use `command`).
    #[serde(default)]
    pub url: String,
    /// Preferred transport, defaults to SSE as per spec
    #[serde(default)]
    pub transport: McpTransport,
    /// Optional static headers (e.g., auth tokens)
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// argv for [`McpTransport::Stdio`] (program + args).
    #[serde(default)]
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum McpTransport {
    #[default]
    Sse,
    Http,
    /// Subprocess MCP (newline-delimited JSON-RPC on stdin/stdout).
    Stdio,
}
