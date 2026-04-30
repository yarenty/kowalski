pub mod agent;
pub mod config;
pub mod conversation;
pub mod db;
pub mod error;
pub mod federation;
pub mod graph;
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

pub use agent::repl_trace::ReplTraceGuard;
pub use agent::{Agent, BaseAgent, MessageHandler};
pub use config::*;
// pub use conversation::*; // Remove this to avoid ToolCall ambiguity
pub use error::KowalskiError;
pub use federation::{
    ABSOLUTE_MAX_DELEGATION_DEPTH, AclEnvelope, AclMessage, AgentRecord, AgentRegistry,
    DEFAULT_MAX_DELEGATION_DEPTH, DelegationOutcome, FederationOrchestrator, MessageBroker,
    MpscBroker, check_delegate_depth, delete_federation_agent, load_registry_into,
    mark_stale_agents_inactive, set_agent_current_task, touch_agent_heartbeat,
    upsert_agent_state_for_record, upsert_registry_record,
};
#[cfg(feature = "postgres")]
pub use federation::{AgentStateSnapshot, load_agent_states};
#[cfg(feature = "postgres")]
pub use federation::{
    PgBroker, bridge_postgres_notify_to_mpsc, bridge_postgres_notify_to_mpsc_pool, pg_pool_connect,
};
pub use graph::{postgres_age_cypher, postgres_graph_status};
pub use logging::*;
pub use mcp::{
    CallToolResponse, McpClient, McpConnection, McpHub, McpStdioClient, McpToolBinding,
    McpToolDescription, McpToolProxy,
};
pub use model::ModelManager;
pub use model::*;
pub use role::{Audience, Preset, Role, Style};
pub use tool_chain::*;
pub use tools::ToolCall;
pub use tools::*;
