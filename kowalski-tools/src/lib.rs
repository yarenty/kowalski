pub mod tool;
pub mod web;
pub mod document;

pub use tool::{Tool, ToolInput, ToolOutput, ToolParameter, ToolError};

/// Common types and utilities used across tools
pub mod types {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ToolMetadata {
        pub name: String,
        pub description: String,
        pub parameters: Vec<ToolParameter>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ToolParameter {
        pub name: String,
        pub description: String,
        pub required: bool,
        pub default_value: Option<String>,
        pub parameter_type: ParameterType,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum ParameterType {
        String,
        Number,
        Boolean,
        Array,
        Object,
    }
}

