use kowalski_web_agent::WebAgent;
use kowalski_core::config::Config;
use log::info;

/// Web agent-specific functionality
pub struct WebMode {
    pub agent: WebAgent,
}

impl WebMode {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let agent = WebAgent::new(config)?;
        Ok(Self { agent })
    }

    /// Start a web-focused chat session
    pub async fn chat_loop(&mut self, model: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting web chat session with model: {}", model);
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
                    println!("  /search - Perform a web search");
                    println!("  /browse - Browse a webpage");
                }
                "/model" => {
                    println!("Current model: {}", model);
                }
                "/search" => {
                    println!("Enter search query:");
                    buffer.clear();
                    stdin.read_line(&mut buffer)?;
                    let query = buffer.trim();
                    let response = self.agent.search(&conv_id, query).await?;
                    println!("{}", response);
                }
                "/browse" => {
                    println!("Enter URL to browse:");
                    buffer.clear();
                    stdin.read_line(&mut buffer)?;
                    let url = buffer.trim();
                    let response = self.agent.browse(&conv_id, url).await?;
                    println!("{}", response);
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

    /// Perform a web search
    pub async fn search(&mut self, conv_id: &str, query: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Performing web search: {}", query);
        let response = self.agent.search(conv_id, query).await?;
        println!("{}", response);
        Ok(())
    }

    /// Browse a webpage
    pub async fn browse(&mut self, conv_id: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Browsing webpage: {}", url);
        let response = self.agent.browse(conv_id, url).await?;
        println!("{}", response);
        Ok(())
    }
} 