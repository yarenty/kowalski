pub mod agent;
pub mod config;
pub mod error;

pub use agent::WebAgent;
pub use config::WebAgentConfig;
pub use error::WebAgentError;

// Re-export common types
pub use kowalski_core::config::Config;
pub use kowalski_core::error::KowalskiError;
pub use kowalski_core::logging;
