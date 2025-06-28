pub mod agent;
pub mod config;
pub mod error;

pub use agent::AcademicAgent;
pub use config::AcademicAgentConfig;

// Re-export common types
pub use kowalski_core::config::Config;
pub use kowalski_core::error::KowalskiError;
pub use kowalski_core::logging;
