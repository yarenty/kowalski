use async_trait::async_trait;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput, ToolParameter};
use serde_json::json;
use std::collections::HashMap;

/// A tool for processing CSV files
pub struct CsvTool {
    max_rows: usize,
    max_columns: usize,
}

impl CsvTool {
    pub fn new(max_rows: usize, max_columns: usize) -> Self {
        Self {
            max_rows,
            max_columns,
        }
    }

    fn read_csv(&self, content: &str) -> Result<serde_json::Value, KowalskiError> {
        let mut rdr = csv::Reader::from_reader(content.as_bytes());
        let headers = rdr
            .headers()
            .map_err(|e| {
                KowalskiError::ContentProcessing(format!("Failed to read CSV headers: {}", e))
            })?
            .iter()
            .map(|h| h.to_string())
            .collect::<Vec<_>>();

        let mut records = Vec::new();
        let mut row_count = 0;

        for result in rdr.records() {
            if row_count >= self.max_rows {
                break;
            }

            let record = result.map_err(|e| {
                KowalskiError::ContentProcessing(format!("Failed to read CSV record: {}", e))
            })?;

            let mut row = HashMap::new();
            for (i, field) in record.iter().enumerate() {
                if i >= self.max_columns {
                    break;
                }
                if i < headers.len() {
                    row.insert(headers[i].clone(), field.to_string());
                } else {
                    row.insert(format!("column_{}", i), field.to_string());
                }
            }
            records.push(row);
            row_count += 1;
        }

        let summary = self.generate_summary(&headers, &records);

        Ok(json!({
            "headers": headers,
            "records": records,
            "summary": summary,
            "total_rows": row_count,
            "total_columns": headers.len()
        }))
    }

    fn generate_summary(
        &self,
        headers: &[String],
        records: &[HashMap<String, String>],
    ) -> serde_json::Value {
        if records.is_empty() {
            return json!({
                "message": "No data to analyze",
                "row_count": 0,
                "column_count": headers.len()
            });
        }

        let mut summary = HashMap::new();
        summary.insert("row_count".to_string(), json!(records.len()));
        summary.insert("column_count".to_string(), json!(headers.len()));

        // Analyze each column
        let mut column_analysis = HashMap::new();
        for header in headers {
            let values: Vec<&String> = records.iter().filter_map(|row| row.get(header)).collect();

            if !values.is_empty() {
                let analysis = self.analyze_column(values);
                column_analysis.insert(header.clone(), analysis);
            }
        }
        summary.insert("columns".to_string(), json!(column_analysis));

        json!(summary)
    }

    fn analyze_column(&self, values: Vec<&String>) -> serde_json::Value {
        let mut analysis = HashMap::new();
        analysis.insert("count".to_string(), json!(values.len()));

        // Try to parse as numbers
        let numbers: Vec<f64> = values
            .iter()
            .filter_map(|v| v.parse::<f64>().ok())
            .collect();

        if !numbers.is_empty() {
            let min = numbers.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max = numbers.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let sum: f64 = numbers.iter().sum();
            let avg = sum / numbers.len() as f64;

            analysis.insert("type".to_string(), json!("numeric"));
            analysis.insert("min".to_string(), json!(min));
            analysis.insert("max".to_string(), json!(max));
            analysis.insert("sum".to_string(), json!(sum));
            analysis.insert("average".to_string(), json!(avg));
        } else {
            // Analyze as text
            let unique_values: std::collections::HashSet<&String> =
                values.iter().cloned().collect();
            analysis.insert("type".to_string(), json!("text"));
            analysis.insert("unique_count".to_string(), json!(unique_values.len()));

            // Most common value
            let mut value_counts: HashMap<&String, usize> = HashMap::new();
            for value in values {
                *value_counts.entry(value).or_insert(0) += 1;
            }

            if let Some((most_common, count)) = value_counts.iter().max_by_key(|&(_, &count)| count)
            {
                analysis.insert("most_common".to_string(), json!(*most_common));
                analysis.insert("most_common_count".to_string(), json!(*count));
            }
        }

        json!(analysis)
    }
}

#[async_trait]
impl Tool for CsvTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        let task = input
            .parameters
            .get("task")
            .and_then(|v| v.as_str())
            .unwrap_or(&input.task_type);

        let content = input
            .parameters
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or(&input.content);

        if content.is_empty() {
            return Err(KowalskiError::ToolExecution(
                "Missing 'content' parameter".to_string(),
            ));
        }

        match task {
            "process_csv" => {
                let result = self.read_csv(content)?;
                Ok(ToolOutput::new(
                    result,
                    Some(json!({
                        "tool": "csv_tool",
                        "max_rows": self.max_rows,
                        "max_columns": self.max_columns
                    })),
                ))
            }
            "analyze_csv" => {
                let result = self.read_csv(content)?;
                let summary = result["summary"].clone();
                Ok(ToolOutput::new(
                    summary,
                    Some(json!({
                        "tool": "csv_tool",
                        "analysis_type": "summary"
                    })),
                ))
            }
            _ => Err(KowalskiError::ToolExecution(format!(
                "Unsupported task type: {}",
                task
            ))),
        }
    }

    fn name(&self) -> &str {
        "csv_tool"
    }

    fn description(&self) -> &str {
        "A tool for processing and analyzing CSV files with statistical summaries"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "task".to_string(),
                description: "The task to perform: 'process_csv' or 'analyze_csv'".to_string(),
                required: true,
                default_value: None,
                parameter_type: kowalski_core::tools::ParameterType::String,
            },
            ToolParameter {
                name: "content".to_string(),
                description: "CSV content to process".to_string(),
                required: true,
                default_value: None,
                parameter_type: kowalski_core::tools::ParameterType::String,
            },
            ToolParameter {
                name: "max_rows".to_string(),
                description: "Maximum number of rows to process".to_string(),
                required: false,
                default_value: Some("1000".to_string()),
                parameter_type: kowalski_core::tools::ParameterType::Number,
            },
            ToolParameter {
                name: "max_columns".to_string(),
                description: "Maximum number of columns to process".to_string(),
                required: false,
                default_value: Some("50".to_string()),
                parameter_type: kowalski_core::tools::ParameterType::Number,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_csv_tool_creation() {
        let tool = CsvTool::new(100, 10);
        assert_eq!(tool.name(), "csv_tool");
    }

    #[tokio::test]
    async fn test_csv_tool_parameters() {
        let tool = CsvTool::new(100, 10);
        let params = tool.parameters();
        assert!(!params.is_empty());
        assert_eq!(params[0].name, "task");
    }
}
