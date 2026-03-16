use futures::StreamExt;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_data_agent::DataAgent;
use tokio::time::Instant;
use std::fs::File;
use std::io::Write;
use std::env;

async fn process_stream_response(mut response: reqwest::Response) -> Result<String, String> {
    let mut buffer = String::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                let text = String::from_utf8(bytes.to_vec())
                    .map_err(|e| format!("Invalid UTF-8: {}", e))?;
                let stream_response: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {}", e))?;
                if let Some(message_val) = stream_response.get("message") {
                    if let Some(content) = message_val.get("content") {
                        buffer.push_str(content.as_str().unwrap_or(""));
                    }
                }
            }
            Err(e) => return Err(format!("Stream error: {}", e)),
        }
    }
    Ok(buffer)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let config = Config::default();
    let mut data_agent = DataAgent::new(config).await?;
    let conversation_id = data_agent.start_conversation("llama3.2");
    let csv_data = r#"name,age,city,salary,department
John Doe,30,New York,75000,Engineering
Jane Smith,28,San Francisco,85000,Marketing
Bob Johnson,35,Chicago,65000,Sales
Alice Brown,32,Boston,70000,Engineering
Charlie Wilson,29,Seattle,80000,Engineering
Diana Davis,31,Austin,72000,Marketing
Eve Miller,27,Denver,68000,Sales
Frank Garcia,33,Portland,75000,Engineering
Grace Lee,26,Atlanta,65000,Marketing
Henry Taylor,34,Dallas,78000,Engineering"#;
    // Write CSV data to a temporary file
    let temp_dir = env::temp_dir();
    let temp_path = temp_dir.join("kowalski_benchmark_s4.csv");
    let mut file = File::create(&temp_path)?;
    file.write_all(csv_data.as_bytes())?;
    let start_time = Instant::now();
    let analysis_result = data_agent.process_csv_path(temp_path.to_str().unwrap()).await?;
    let analysis_prompt = format!(
        "Analyze this data and provide insights:\n\n{}\n\nAnalysis results:\n{}",
        csv_data,
        serde_json::to_string_pretty(&analysis_result)?
    );
    let response = data_agent
        .chat_with_history(&conversation_id, &analysis_prompt, None)
        .await?;
    let full_response = process_stream_response(response).await?;
    let elapsed = start_time.elapsed();
    println!("Kowalski (CSV Analysis) - Response: {}", full_response);
    println!("Kowalski (CSV Analysis) - Time: {:?}", elapsed);
    Ok(())
}
