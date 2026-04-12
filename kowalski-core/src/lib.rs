pub mod agent;
pub mod config;
pub mod db;
pub mod federation;
pub mod conversation;
pub mod error;
pub mod llm;
pub mod logging;
pub mod mcp;
pub mod memory;
pub mod model;
pub mod role;
pub mod template;
pub mod tool_chain;
pub mod tools;
pub mod utils;

pub use agent::{Agent, BaseAgent, MessageHandler};
pub use config::*;
// pub use conversation::*; // Remove this to avoid ToolCall ambiguity
pub use error::KowalskiError;
pub use federation::{
    check_delegate_depth, AclEnvelope, AclMessage, AgentRecord, AgentRegistry, DelegationOutcome,
    FederationOrchestrator, MessageBroker, MpscBroker,
};
#[cfg(feature = "postgres")]
pub use federation::{
    bridge_postgres_notify_to_mpsc, bridge_postgres_notify_to_mpsc_pool, pg_pool_connect, PgBroker,
};
pub use logging::*;
pub use mcp::{
    CallToolResponse, McpClient, McpHub, McpToolBinding, McpToolDescription, McpToolProxy,
};
pub use model::ModelManager;
pub use model::*;
pub use role::{Audience, Preset, Role, Style};
pub use tool_chain::*;
pub use tools::ToolCall;
pub use tools::*;
