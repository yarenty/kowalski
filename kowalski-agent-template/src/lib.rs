pub mod agent;
pub mod builder;
pub mod config;

pub mod default;

pub use agent::TemplateAgent;
pub use config::TemplateAgentConfig;

// Re-export common types
pub use kowalski_core::config::Config;
pub use kowalski_core::error::KowalskiError;
pub use kowalski_core::logging;
