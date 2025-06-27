pub mod agent;
pub mod registry;
pub mod message;
pub mod error;

pub use agent::{FederatedAgent, FederationRole};
pub use registry::AgentRegistry;
pub use message::{FederationMessage, MessageType};
pub use error::FederationError;

/// Re-export common types from core
pub use kowalski_core::{
    Agent,
    BaseAgent,
    Config,
    Message,
    Role,
    TaskType,
    ToolInput,
    ToolOutput,
};
