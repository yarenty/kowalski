pub mod agent;
pub mod config;
pub mod tools;

pub use agent::TemplateAgent;
pub use config::TemplateAgentConfig;

// Re-export common types
pub use kowalski_core::config::Config;
pub use kowalski_core::logging; 
pub use kowalski_core::error::KowalskiError;