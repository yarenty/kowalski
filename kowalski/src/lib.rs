#![doc = include_str!("../README.md")]

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
