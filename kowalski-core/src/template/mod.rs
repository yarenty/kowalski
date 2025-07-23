pub mod agent;
pub mod builder;
pub mod config;

pub mod default;

pub use agent::TemplateAgent;
pub use config::TemplateAgentConfig;

// Re-export common types
pub use crate::config::Config;
pub use crate::error::KowalskiError;
pub use crate::logging;
