pub mod agent;
pub mod error;
pub mod message;
pub mod registry;

pub use agent::{FederatedAgent, FederationRole};
pub use error::FederationError;
pub use message::{FederationMessage, MessageType};
pub use registry::AgentRegistry;

/// Re-export common types from core
pub use kowalski_core::{Agent, BaseAgent, Config, Message, Role, TaskType, ToolInput, ToolOutput};
