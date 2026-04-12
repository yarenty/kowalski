use crate::error::KowalskiError;
use crate::mcp::hub::McpHub;
use crate::mcp::types::McpToolDescription;
use crate::tools::{ParameterType, Tool, ToolInput, ToolOutput, ToolParameter};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub struct McpToolProxy {
    hub: Arc<McpHub>,
    name: String,
    description: String,
    parameters: Vec<ToolParameter>,
}

impl McpToolProxy {
    pub fn new(hub: Arc<McpHub>, display_name: String, description: McpToolDescription) -> Self {
        let parameters = Self::schema_to_parameters(&description.input_schema);
        Self {
            hub,
            name: display_name,
            description: description.description,
            parameters,
        }
    }

    fn schema_to_parameters(schema: &Value) -> Vec<ToolParameter> {
        let mut params = Vec::new();
        let required = schema
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
            for (name, details) in props {
                let description = details
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No description provided")
                    .to_string();
                let required_flag = required.iter().any(|req| req == name);
                let default_value = details.get("default").map(|v| v.to_string());
                let param_type = details
                    .get("type")
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "number" => ParameterType::Number,
                        "boolean" => ParameterType::Boolean,
                        "array" => ParameterType::Array,
                        "object" => ParameterType::Object,
                        _ => ParameterType::String,
                    })
                    .unwrap_or(ParameterType::String);

                params.push(ToolParameter {
                    name: name.clone(),
                    description,
                    required: required_flag,
                    default_value,
                    parameter_type: param_type,
                });
            }
        }

        params
    }
}

#[async_trait]
impl Tool for McpToolProxy {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        self.validate_input(&input)?;
        let value = self.hub.call_tool(&self.name, &input.parameters).await?;
        Ok(ToolOutput::new(value, None))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        self.parameters.clone()
    }
}
