use super::ToolError;
use crate::tool::{Tool, ToolInput, ToolOutput, ToolParameter, ParameterType};
use async_trait::async_trait;
use lopdf::{Document, Object, ObjectId};
use serde_json::json;
use std::fs;
use std::path::Path;

pub struct PdfTool;

#[async_trait]
impl Tool for PdfTool {
    fn name(&self) -> &str {
        "pdf_process"
    }

    fn description(&self) -> &str {
        "Processes PDF files to extract text and metadata"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "file_path".to_string(),
                description: "Path to the PDF file to process".to_string(),
                required: true,
                default_value: None,
                parameter_type: ParameterType::String,
            },
            ToolParameter {
                name: "extract_text".to_string(),
                description: "Whether to extract text content".to_string(),
                required: false,
                default_value: Some("true".to_string()),
                parameter_type: ParameterType::Boolean,
            },
            ToolParameter {
                name: "extract_metadata".to_string(),
                description: "Whether to extract PDF metadata".to_string(),
                required: false,
                default_value: Some("true".to_string()),
                parameter_type: ParameterType::Boolean,
            },
            ToolParameter {
                name: "extract_images".to_string(),
                description: "Whether to extract images from PDF".to_string(),
                required: false,
                default_value: Some("false".to_string()),
                parameter_type: ParameterType::Boolean,
            },
        ]
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let params = input.parameters.as_object().ok_or_else(|| {
            ToolError::InvalidInput("Input parameters must be a JSON object".to_string())
        })?;

        let file_path = params
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing required parameter: file_path".to_string()))?
            .to_string();

        let extract_text = params
            .get("extract_text")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let extract_metadata = params
            .get("extract_metadata")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let extract_images = params
            .get("extract_images")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let result = self.process_pdf(&file_path, extract_text, extract_metadata, extract_images)?;

        Ok(ToolOutput {
            result: json!(result),
            metadata: Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "file_path": file_path,
                "extract_text": extract_text,
                "extract_metadata": extract_metadata,
                "extract_images": extract_images,
            })),
        })
    }

    fn validate_input(&self, input: &ToolInput) -> Result<(), ToolError> {
        // Additional validation for PDF processing
        let params = input.parameters.as_object().ok_or_else(|| {
            ToolError::InvalidInput("Input parameters must be a JSON object".to_string())
        })?;

        let file_path = params
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing required parameter: file_path".to_string()))?
            .to_string();

        if !Path::new(&file_path).exists() {
            return Err(ToolError::InvalidInput(format!(
                "File does not exist: {}",
                file_path
            )));
        }

        if !file_path.ends_with(".pdf") {
            return Err(ToolError::InvalidInput("File must be a PDF".to_string()));
        }

        Ok(())
    }
}

impl PdfTool {
    fn process_pdf(
        &self,
        file_path: &str,
        extract_text: bool,
        extract_metadata: bool,
        extract_images: bool,
    ) -> Result<serde_json::Value, ToolError> {
        let bytes = fs::read(file_path)
            .map_err(|e| ToolError::Execution(format!("Failed to read file: {}", e)))?;

        let mut doc = Document::load_mem(&bytes)
            .map_err(|e| ToolError::Execution(format!("Failed to parse PDF: {}", e)))?;

        let mut result = serde_json::Map::new();

        if extract_metadata {
            let metadata = self.extract_metadata(&doc)?;
            result.insert("metadata".to_string(), json!(metadata));
        }

        if extract_text {
            let text = self.extract_text(&doc)?;
            result.insert("text".to_string(), json!(text));
        }

        if extract_images {
            let images = self.extract_images(&doc)?;
            result.insert("images".to_string(), json!(images));
        }

        Ok(serde_json::Value::Object(result))
    }

    fn extract_metadata(&self, doc: &Document) -> Result<serde_json::Map<String, serde_json::Value>, ToolError> {
        let mut metadata = serde_json::Map::new();

        if let Some(info) = doc.trailer.info {
            for (key, value) in doc.get_object(info) {
                if let Ok(Object::String(s)) = value.as_string() {
                    metadata.insert(key.to_string(), json!(s));
                }
            }
        }

        Ok(metadata)
    }

    fn extract_text(&self, doc: &Document) -> Result<String, ToolError> {
        let mut text = String::new();

        for page_id in doc.get_pages() {
            let page = doc.get_page(page_id).map_err(|e| ToolError::Execution(format!("Failed to get page: {}", e)))?;
            let content = page.get_content()?
                .iter()
                .filter_map(|obj| match obj {
                    Object::String(s) => Some(s.to_string()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ");

            text.push_str(&content);
            text.push('\n');
        }

        Ok(text)
    }

    fn extract_images(&self, doc: &Document) -> Result<Vec<String>, ToolError> {
        let mut images = Vec::new();

        for page_id in doc.get_pages() {
            let page = doc.get_page(page_id).map_err(|e| ToolError::Execution(format!("Failed to get page: {}", e)))?;
            
            // Extract images from page content
            // Implementation would depend on the specific PDF structure
            // This is a placeholder for actual image extraction logic
        }

        Ok(images)
    }
}
