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