//! Kowalski - A Rust-based agent framework for interacting with Ollama models
//! 
//! This crate provides a comprehensive framework for building AI agents with various capabilities:
//! 
//! ## Core Components
//! - **Core**: Basic agent infrastructure and types
//! - **Agent Template**: Templates for building custom agents
//! - **Tools**: Various tools for web scraping, data processing, and more
//! - **Federation**: Multi-agent coordination and communication
//! 
//! ## Specialized Agents
//! - **Academic Agent**: Research and academic paper analysis
//! - **Code Agent**: Code analysis, refactoring, and generation
//! - **Data Agent**: Data analysis and processing (optional feature)
//! - **Web Agent**: Web research and information gathering
//! 
//! ## Usage
//! 
//! ```rust
//! use kowalski::core::BaseAgent;
//! use kowalski::agent_template::AgentBuilder;
//! 
//! // Create a basic agent
//! let agent = AgentBuilder::new()
//!     .with_model("llama2")
//!     .build()?;
//! ```
//! 
//! ## Features
//! 
//! - `data`: Enable data analysis capabilities (includes kowalski-data-agent)

// Re-export core functionality
pub use kowalski_core as core;

// Re-export agent template
pub use kowalski_agent_template as agent_template;

// Re-export tools
pub use kowalski_tools as tools;

// Re-export federation
pub use kowalski_federation as federation;

// Re-export specialized agents
pub use kowalski_academic_agent as academic_agent;
pub use kowalski_code_agent as code_agent;
pub use kowalski_web_agent as web_agent;
pub use kowalski_data_agent as data_agent;

// Convenience re-exports for common types
pub use kowalski_core::{
    BaseAgent,
    Config,
    Conversation,
    Message,
    Role,
    Tool,
    ToolChain,
};

pub use kowalski_agent_template::builder::AgentBuilder;

// Re-export error types
pub use kowalski_core::error::KowalskiError;
pub type Result<T> = std::result::Result<T, KowalskiError>;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION"); 