//! Kowalski - A Rust-based agent framework for interacting with Ollama models
//!
//! This crate provides a comprehensive framework for building AI agents with various capabilities.
//! It acts as a facade, re-exporting functionality from the other crates in the `kowalski` workspace.
//!
//! ## Core components
//! - **`kowalski_core`**: Re-exported as `kowalski::core` — `TemplateAgent`, tools, memory, MCP, federation types.
//! - **Tools** live inside `kowalski-core` (not a separate `kowalski-tools` crate).
//!
//! ## Optional features
//! - **`cli`**: `kowalski-cli` as `kowalski::cli`
//! - **`postgres`**: Postgres / pgvector paths in `kowalski-core`
//! - **`full`**: `cli` + `postgres`
//!
//! ## Usage
//!
//! Add `kowalski` to your `Cargo.toml` and enable the features you need (see the workspace `Cargo.toml` for the current version).
//!
//! ```toml
//! [dependencies]
//! kowalski = { version = "1.1.0" }
//! ```
//!
//! ```rust,no_run
//! use kowalski::core::agent::{Agent, BaseAgent};
//! use kowalski::core::template::builder::AgentBuilder;
//! use kowalski::core::config::Config;
//! use kowalski::core::error::KowalskiError;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), KowalskiError> {
//!     // Create a basic agent
//!     let agent = AgentBuilder::new().await
//!         .build().await?;
//!     Ok(())
//! }
//! ```

// Re-export core components

pub use kowalski_core as core;

// Re-export optional CLI
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
