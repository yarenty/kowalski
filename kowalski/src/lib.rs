//! Kowalski - A Rust-based agent framework for interacting with Ollama models
//!
//! This crate provides a comprehensive framework for building AI agents with various capabilities.
//! It acts as a facade, re-exporting functionality from the other crates in the `kowalski` workspace.
//!
//! ## Core Components
//! - **Core**: Basic agent infrastructure and types (`kowalski-core`)
//! - **Memory**: Multi-tiered memory system (`kowalski-memory`)
//! - **Agent Template**: Templates for building custom agents (`kowalski-agent-template`)
//! - **Tools**: Various tools for web scraping, data processing, and more (`kowalski-tools`)
//!
//! ## Optional Features (Specialized Agents and more)
//! - **`academic`**: Research and academic paper analysis (`kowalski-academic-agent`)
//! - **`code`**: Code analysis, refactoring, and generation (`kowalski-code-agent`)
//! - **`data`**: Data analysis and processing (`kowalski-data-agent`)
//! - **`web`**: Web research and information gathering (`kowalski-web-agent`)
//! - **`federation`**: Multi-agent coordination and communication (`kowalski-federation`)
//! - **`cli`**: Command-line interface (`kowalski-cli`)
//!
//! ## Usage
//!
//! Add `kowalski` to your `Cargo.toml` and enable the features you need:
//!
//! ```toml
//! [dependencies]
//! kowalski = { version = "0.5.2", features = ["web", "code"] }
//! ```
//!
//! ```rust,no_run
/
/! use kowalski::core::agent::{Agent, BaseAgent};
//! use kowalski::core::template::builder::AgentBuilder;
//! use kowalski::core::config::Config;
//! use kowalski::core::error::KowalskiError;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), KowalskiError> {
//!     // Create a basic agent
//!     let agent = AgentBuilder::new()
//!         .with_model("llama2")
//!         .build().await?;
//!     Ok(())
//! }
//! ```

// Re-export core components

pub use kowalski_core as core;
pub use kowalski_tools as tools;

// Re-export optional agents
#[cfg(feature = "academic")]
pub use kowalski_academic_agent as academic_agent;

#[cfg(feature = "code")]
pub use kowalski_code_agent as code_agent;

#[cfg(feature = "data")]
pub use kowalski_data_agent as data_agent;

#[cfg(feature = "web")]
pub use kowalski_web_agent as web_agent;

// Re-export optional federation and CLI
#[cfg(feature = "federation")]
pub use kowalski_federation as federation;

#[cfg(feature = "cli")]
pub use kowalski_cli as cli;

// Convenience re-exports for common types
pub use crate::core::{
    agent::{Agent, BaseAgent},
    config::Config,
    conversation::{Conversation, Message},
    memory::episodic::EpisodicBuffer,
    memory::semantic::SemanticStore,
    memory::{MemoryProvider, MemoryUnit, working::WorkingMemory},
    role::Role,
    template::TemplateAgent,
    template::builder::AgentBuilder,
    template::default::DefaultTemplate,
    tool_chain::ToolChain,
    tools::{ParameterType, Tool, ToolCall, ToolInput, ToolOutput, ToolParameter},
};

// Re-export error types
pub use crate::core::error::KowalskiError;
pub type Result<T> = std::result::Result<T, KowalskiError>;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
