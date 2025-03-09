pub mod agent;
pub mod config;
pub mod conversation;
pub mod model;
pub mod role;
pub mod utils;
pub mod tools;

// Re-export commonly used types
pub use agent::{Agent, AcademicAgent, ToolingAgent};
pub use config::Config;
pub use model::ModelManager;
pub use role::{Role, Audience, Preset}; 