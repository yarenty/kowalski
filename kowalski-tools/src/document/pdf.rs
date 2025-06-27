use async_trait::async_trait;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{ParameterType, ToolParameter};
use kowalski_core::tools::{Tool, ToolInput, ToolOutput};
use lopdf::{Document, Object};
use serde_json::json;

pub struct PdfTool;

#[async_trait]
impl Tool for PdfTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        // Parse parameters from input
        let file_path = input
            .parameters
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                KowalskiError::ToolExecution("Missing or invalid 'file_path' parameter".to_string())
            })?;
        let extract_text = input
            .parameters
            .get("extract_text")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let extract_metadata = input
            .parameters
            .get("extract_metadata")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let extract_images = input
            .parameters
            .get("extract_images")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let result = self.process_pdf(file_path, extract_text, extract_metadata, extract_images)?;
        Ok(ToolOutput {
            result,
            metadata: None,
        })
    }

    fn name(&self) -> &str {
        "pdf_tool"
    }

    fn description(&self) -> &str {
        "Processes PDF files to extract text, metadata, and images."
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "file_path".to_string(),
                description: "Path to the PDF file to process.".to_string(),
                required: true,
                default_value: None,
                parameter_type: ParameterType::String,
            },
            ToolParameter {
                name: "extract_text".to_string(),
                description: "Whether to extract text from the PDF (true/false).".to_string(),
                required: true,
                default_value: Some("true".to_string()),
                parameter_type: ParameterType::Boolean,
            },
            ToolParameter {
                name: "extract_metadata".to_string(),
                description: "Whether to extract metadata from the PDF (true/false).".to_string(),
                required: true,
                default_value: Some("false".to_string()),
                parameter_type: ParameterType::Boolean,
            },
            ToolParameter {
                name: "extract_images".to_string(),
                description: "Whether to extract images from the PDF (true/false).".to_string(),
                required: true,
                default_value: Some("false".to_string()),
                parameter_type: ParameterType::Boolean,
            },
        ]
    }
}

impl PdfTool {
    fn process_pdf(
        &self,
        file_path: &str,
        extract_text: bool,
        extract_metadata: bool,
        extract_images: bool,
    ) -> Result<serde_json::Value, KowalskiError> {
        let doc = Document::load(file_path)
            .map_err(|e| KowalskiError::Execution(format!("Failed to parse PDF: {}", e)))?;

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

    fn extract_metadata(
        &self,
        doc: &Document,
    ) -> Result<serde_json::Map<String, serde_json::Value>, KowalskiError> {
        let mut metadata = serde_json::Map::new();

        if let Ok(Object::Reference(info_id)) = doc.trailer.get(b"Info") {
            if let Ok(obj) = doc.get_object(*info_id) {
                if let Ok(info_dict) = obj.as_dict() {
                    for (key, value) in info_dict.iter() {
                        let key_vec = key.to_vec();
                        let key_str = String::from_utf8_lossy(&key_vec).to_string();
                        if let lopdf::Object::String(ref s, _) = value {
                            let val_str = String::from_utf8_lossy(s).to_string();
                            metadata.insert(key_str, json!(val_str));
                        }
                    }
                }
            }
        }

        Ok(metadata)
    }

    fn extract_text(&self, doc: &Document) -> Result<String, KowalskiError> {
        let mut text = String::new();
        let pages = doc.get_pages();
        for (page_number, page_id) in pages.iter() {
            // Placeholder: just output the page number and object ID
            let page_text = format!("Page {} object ID: {:?}", page_number, page_id);
            text.push_str(&page_text);
            text.push('\n');
        }
        Ok(text)
    }

    fn extract_images(&self, _doc: &Document) -> Result<Vec<String>, KowalskiError> {
        let images = Vec::new();
        // Placeholder for actual image extraction logic
        Ok(images)
    }
}
