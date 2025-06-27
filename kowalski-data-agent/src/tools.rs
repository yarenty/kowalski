use async_trait::async_trait;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput};
use serde_json::json;
use std::error::Error;

pub struct CsvTool;

#[async_trait]
impl Tool for CsvTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
        if input.task_type == "process_csv" {
            let content = input.content.as_str();
            match read_csv(content) {
                Ok(json_output) => Ok(ToolOutput::new(json_output, None)),
                Err(e) => Err(e.to_string()),
            }
        } else {
            Err("Task type not supported".to_string())
        }
    }
}

fn read_csv(content: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    let mut rdr = csv::Reader::from_reader(content.as_bytes());
    let mut records = Vec::new();
    for result in rdr.records() {
        let record = result?;
        records.push(record);
    }
    let json_output = json!({
        "headers": rdr.headers()?,
        "records": records
    });
    Ok(json_output)
}
