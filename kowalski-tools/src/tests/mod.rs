use super::tool::{Tool, ToolInput, ToolOutput, ToolParameter};
use super::web::{WebSearchTool, WebScrapeTool};
use super::document::PdfTool;
use serde_json::json;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[tokio::test]
async fn test_web_search_tool() {
    let search_tool = WebSearchTool::new("duckduckgo".to_string());
    
    let input = ToolInput {
        task_type: "web_search".to_string(),
        content: "Rust programming language".to_string(),
        parameters: json!({
            "query": "Rust programming language",
            "num_results": 3
        }),
    };

    let result = search_tool.execute(input).await;
    assert!(result.is_ok());
    
    if let Ok(output) = result {
        assert!(output.result.is_object());
        assert!(output.metadata.is_some());
    }
}

#[tokio::test]
async fn test_web_scrape_tool() {
    let scrape_tool = WebScrapeTool::new();
    
    let input = ToolInput {
        task_type: "web_scrape".to_string(),
        content: "https://www.rust-lang.org".to_string(),
        parameters: json!({
            "url": "https://www.rust-lang.org",
            "selectors": ["h1", "p"],
            "follow_links": false,
            "max_depth": 1
        }),
    };

    let result = scrape_tool.execute(input).await;
    assert!(result.is_ok());
    
    if let Ok(output) = result {
        assert!(output.result.is_array());
        assert!(output.metadata.is_some());
    }
}

#[tokio::test]
fn test_pdf_tool() {
    // Create a temporary PDF file for testing
    let temp_dir = tempfile::tempdir().unwrap();
    let pdf_path = temp_dir.path().join("test.pdf");
    
    // Create a simple PDF with some text
    let mut doc = lopdf::Document::with_version("1.5");
    let page_id = doc.add_page(lopdf::Object::Stream(lopdf::Stream {
        content: vec![],
        dictionary: lopdf::Dictionary::new(),
    }));
    doc.set_page_content(page_id, vec![]);
    
    // Save to temporary file
    doc.save(&pdf_path).unwrap();
    
    let pdf_tool = PdfTool;
    
    let input = ToolInput {
        task_type: "pdf_process".to_string(),
        content: pdf_path.to_str().unwrap().to_string(),
        parameters: json!({
            "file_path": pdf_path.to_str().unwrap(),
            "extract_text": true,
            "extract_metadata": true,
            "extract_images": false
        }),
    };

    // Run in a new runtime since this is a blocking test
    let rt = Runtime::new().unwrap();
    let result = rt.block_on(pdf_tool.execute(input));
    
    assert!(result.is_ok());
    
    if let Ok(output) = result {
        assert!(output.result.is_object());
        assert!(output.metadata.is_some());
    }
}
