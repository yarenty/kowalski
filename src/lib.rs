// Re-export commonly used types and core functionality
pub use kowalski_core::{
    config::Config,
    error::KowalskiError,
    logging,
    model::ModelManager,
    role::{Audience, Preset, Role, Style},
    // ... any other needed re-exports
};

// Re-export core functionality
pub use kowalski_core::*;
