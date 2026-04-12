use colored::*;
use kowalski_core::agent::Agent;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};

pub struct InteractiveSession {
    agent: Box<dyn Agent + Send + Sync>,
    conversation_id: String,
}

impl InteractiveSession {
    pub fn new(mut agent: Box<dyn Agent + Send + Sync>, model: &str) -> Self {
        let conversation_id = agent.start_conversation(model);
        Self {
            agent,
            conversation_id,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut rl = DefaultEditor::new()?;
        let history_path = "history.txt";

        if rl.load_history(history_path).is_err() {
            println!("No previous history.");
        }

        println!("{}", "Welcome to Kowalski Interactive Mode!".green().bold());
        println!("{}", "Type your message or use /help for commands.".cyan());

        loop {
            let readline = rl.readline(">> ");
            match readline {
                Ok(line) => {
                    let trim_line = line.trim();
                    if trim_line.is_empty() {
                        continue;
                    }

                    rl.add_history_entry(trim_line)?;

                    if trim_line.starts_with('/') {
                        let cmd = trim_line.to_lowercase();
                        match cmd.as_str() {
                            "/exit" | "/quit" => {
                                println!("{}", "Goodbye!".yellow());
                                break;
                            }
                            "/help" => {
                                self.print_help();
                                continue;
                            }
                            "/clear" => {
                                // Simplified clear
                                print!("\x1B[2J\x1B[1;1H");
                                continue;
                            }
                            _ => {
                                println!("{} {}", "Unknown command:".red(), cmd);
                                continue;
                            }
                        }
                    }

                    // Process with agent
                    println!("{}", "Processing...".italic().dimmed());
                    match self
                        .agent
                        .chat_with_tools(&self.conversation_id, trim_line)
                        .await
                    {
                        Ok(response) => {
                            println!("\n{}\n", response.blue());
                        }
                        Err(e) => {
                            println!("{} {}", "Error:".red().bold(), e);
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Interrupted");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!("EOF");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }

        rl.save_history(history_path)?;
        Ok(())
    }

    fn print_help(&self) {
        println!("{}", "\nAvailable Commands:".bold());
        println!("  {} - Display this help message", "/help".cyan());
        println!("  {} - Clear the screen", "/clear".cyan());
        println!("  {} - Exit the interactive session", "/exit".cyan());
        println!("  {} - Exit the interactive session", "/quit".cyan());
        println!();
    }
}
