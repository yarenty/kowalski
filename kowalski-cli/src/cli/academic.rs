use kowalski_academic_agent::AcademicAgent;
use kowalski_core::config::Config;
use std::path::PathBuf;
use log::info;

/// Academic agent-specific functionality
pub struct AcademicMode {
    pub agent: AcademicAgent,
}

impl AcademicMode {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let agent = AcademicAgent::new(config)?;
        Ok(Self { agent })
    }

    /// Process an academic paper
    pub async fn process_paper(&mut self, file: &PathBuf, model: &str, format: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Processing academic paper: {:?}", file);
        let conv_id = self.agent.create_conversation(model)?;
        let response = self.agent.process_file(&conv_id, file).await?;
        println!("{}", response);
        Ok(())
    }

    /// Start an academic-focused chat session
    pub async fn chat_loop(&mut self, model: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting academic chat session with model: {}", model);
        println!("Type '/bye' to exit, '/help' for commands");

        let conv_id = self.agent.create_conversation(model)?;
        let mut buffer = String::new();
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();

        loop {
            print!("> ");
            stdout.flush()?;
            buffer.clear();
            stdin.read_line(&mut buffer)?;
            let input = buffer.trim();

            match input {
                "/bye" => break,
                "/help" => {
                    println!("Available commands:");
                    println!("  /bye   - Exit the chat session");
                    println!("  /help  - Show this help message");
                    println!("  /model - Show current model info");
                    println!("  /paper - Process a paper file");
                }
                "/model" => {
                    println!("Current model: {}", model);
                }
                "/paper" => {
                    println!("Enter paper file path:");
                    buffer.clear();
                    stdin.read_line(&mut buffer)?;
                    let file_path = buffer.trim();
                    let path = PathBuf::from(file_path);
                    if path.exists() { {
                        self.process_paper(&path, model, "text").await?;
                    } else {
                        println!("Invalid file path");
                    }
                }
                _ if !input.is_empty() => {
                    let response = self.agent.send_message(&conv_id, input).await?;
                    println!("{}", response);
                }
                _ => continue,
            }
        }

        Ok(())
    }
} 