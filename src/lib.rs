pub mod agent;
pub mod config;
pub mod conversation;
pub mod model;
pub mod role;
pub mod tools;
pub mod utils;

// Re-export commonly used types
pub use agent::{AcademicAgent, Agent, ToolingAgent};
pub use config::Config;
pub use model::ModelManager;
pub use role::{Audience, Preset, Role};
