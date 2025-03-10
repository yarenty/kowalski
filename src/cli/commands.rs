use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Kowalski - Your AI-powered research assistant
/// "Because Googling is so last decade"
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands for Kowalski
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Chat with the AI (because talking to yourself is frowned upon)
    Chat {
        /// Your message to the AI
        message: String,
        /// Model to use (default: llama2)
        #[arg(short, long, default_value = "llama2")]
        model: String,
    },
    /// Analyze academic papers (for when reading is too mainstream)
    Academic {
        /// Path to the research paper
        #[arg(short, long)]
        file: PathBuf,
        /// Model to use (default: llama2)
        #[arg(short, long, default_value = "llama2")]
        model: String,
        /// Output format (default: text)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Manage AI models (like a zookeeper, but for neural networks)
    Model {
        #[command(subcommand)]
        command: ModelCommands,
    },
    /// Use AI-powered tools (for when you need more than just chat)
    Tool {
        #[command(subcommand)]
        command: ToolCommands,
    },
}

/// Commands for managing AI models
#[derive(Subcommand, Debug)]
pub enum ModelCommands {
    /// List available models
    List,
    /// Pull a new model
    Pull {
        /// Name of the model to pull
        name: String,
    },
    /// Remove a model
    Remove {
        /// Name of the model to remove
        name: String,
    },
    /// Show model details
    Show {
        /// Name of the model to show
        name: String,
    },
}

/// Commands for using AI-powered tools
#[derive(Subcommand, Debug)]
pub enum ToolCommands {
    /// Search the web (because Google is so 2023)
    Search {
        /// Your search query
        query: String,
        /// Model to use (default: llama2)
        #[arg(short, long, default_value = "llama2")]
        model: String,
        /// Number of results to return (default: 5)
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },
    /// Scrape and analyze a webpage (for when reading is too mainstream)
    Scrape {
        /// URL to scrape
        url: String,
        /// Model to use (default: llama2)
        #[arg(short, long, default_value = "llama2")]
        model: String,
        /// Whether to follow links (default: false)
        #[arg(short, long, default_value = "false")]
        follow_links: bool,
        /// Maximum depth for link following (default: 1)
        #[arg(short, long, default_value = "1")]
        max_depth: u32,
    },
    /// Analyze code (for when Stack Overflow is down)
    Code {
        /// Path to the code file or directory
        #[arg(short, long)]
        path: PathBuf,
        /// Model to use (default: llama2)
        #[arg(short, long, default_value = "llama2")]
        model: String,
        /// Language of the code (optional)
        #[arg(short, long)]
        language: Option<String>,
    },
}

impl ToolCommands {
    /// Get the model name from the command
    pub fn model(&self) -> &str {
        match self {
            ToolCommands::Search { model, .. } => model,
            ToolCommands::Scrape { model, .. } => model,
            ToolCommands::Code { model, .. } => model,
        }
    }
} 