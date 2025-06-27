use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Kowalski - Your AI-powered research assistant
///
/// A versatile AI agent that can chat, analyze academic papers, and use various tools
/// to help you with research, coding, and web interactions.
///
/// Examples:
///   kowalski chat "What's the meaning of life?"
///   kowalski academic analyze research-paper.pdf
///   kowalski tool search "rust async programming"
///   kowalski model list
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands for Kowalski
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Chat with the AI
    ///
    /// Start a conversation with the AI model. The conversation will continue
    /// until you type '/bye'. The AI will maintain context throughout the chat.
    ///
    /// Examples:
    ///   kowalski chat "Tell me about Rust"
    ///   kowalski chat "What's the best way to learn programming?" --model llama2
    Chat {
        /// Your message to the AI
        #[arg(help = "The message you want to send to the AI")]
        message: String,
        /// Model to use
        #[arg(
            short,
            long,
            default_value = "llama2",
            help = "The AI model to use for the conversation"
        )]
        model: String,
    },
    /// Analyze academic papers
    ///
    /// Upload and analyze academic papers in PDF or text format. The AI will
    /// help you understand the content and answer questions about it.
    ///
    /// Examples:
    ///   kowalski academic --file research.pdf
    ///   kowalski academic --file paper.txt --model llama2
    Academic {
        /// Path to the file to analyze
        #[arg(help = "Path to the academic paper file")]
        file: PathBuf,
        /// Model to use
        #[arg(
            short,
            long,
            default_value = "llama2",
            help = "The AI model to use for analysis"
        )]
        model: String,
        /// Output format
        #[arg(
            short,
            long,
            default_value = "text",
            help = "Output format (text, json, markdown)"
        )]
        format: String,
    },
    /// Manage AI models
    ///
    /// List, download, or remove AI models that can be used with Kowalski.
    ///
    /// Examples:
    ///   kowalski model list
    ///   kowalski model pull llama2
    ///   kowalski model remove llama2
    Model {
        #[command(subcommand)]
        command: ModelCommands,
    },
    /// Use various tools
    ///
    /// Access various AI-powered tools for web search, scraping, and code analysis.
    /// Each tool is specialized for a specific task and can be configured as needed.
    ///
    /// Examples:
    ///   kowalski tool search "rust async programming"
    ///   kowalski tool scrape "https://example.com"
    ///   kowalski tool code ./src/main.rs
    Tool {
        #[command(subcommand)]
        command: ToolCommands,
    },
    /// Analyze data from a file
    #[cfg(feature = "data")]
    Data {
        /// Path to the file to analyze
        #[arg(help = "Path to the data file")]
        file: PathBuf,
        /// Model to use
        #[arg(
            short,
            long,
            default_value = "llama2",
            help = "The AI model to use for analysis"
        )]
        model: String,
    },
}

/// Commands for managing AI models
#[derive(Subcommand, Debug)]
pub enum ModelCommands {
    /// List available models
    ///
    /// Shows all AI models that are currently installed and available for use.
    /// Displays information about each model including size and last modified date.
    ///
    /// Example:
    ///   kowalski model list
    List,
    /// Pull a model from the repository
    ///
    /// Downloads and installs a new AI model. The model will be available for use
    /// after the download is complete.
    ///
    /// Example:
    ///   kowalski model pull llama2
    Pull {
        /// Name of the model to pull
        #[arg(help = "Name of the model to pull")]
        name: String,
    },
    /// Delete a model
    ///
    /// Uninstalls an AI model to free up disk space. The model will need to be
    /// pulled again if you want to use it in the future.
    ///
    /// Example:
    ///   kowalski model remove llama2
    Delete {
        /// Name of the model to delete
        #[arg(help = "Name of the model to delete")]
        name: String,
    },
}

/// Commands for using various tools
#[derive(Subcommand, Debug)]
pub enum ToolCommands {
    /// Search the web
    ///
    /// Performs a web search using AI-powered search capabilities. Results are
    /// processed and summarized by the AI to provide relevant information.
    ///
    /// Examples:
    ///   kowalski tool search "rust async programming"
    ///   kowalski tool search "best practices for error handling" --limit 10
    Search {
        /// Search query
        #[arg(help = "The search query")]
        query: String,
        /// Model to use
        #[arg(short, long, default_value = "llama2", help = "The AI model to use")]
        model: String,
        /// Maximum number of results
        #[arg(
            short,
            long,
            default_value = "10",
            help = "Maximum number of results to return"
        )]
        limit: usize,
    },
    /// Browse a webpage
    ///
    /// Downloads and analyzes the content of a webpage. Can optionally follow
    /// links to gather more information from related pages.
    ///
    /// Examples:
    ///   kowalski tool scrape "https://example.com"
    ///   kowalski tool scrape "https://docs.rs" --follow-links --max-depth 2
    Browse {
        /// URL to browse
        #[arg(help = "The URL to browse")]
        url: String,
        /// Model to use
        #[arg(short, long, default_value = "llama2", help = "The AI model to use")]
        model: String,
        /// Whether to follow links
        #[arg(short, long, help = "Whether to follow links on the page")]
        follow_links: bool,
        /// Maximum depth for following links
        #[arg(
            short,
            long,
            default_value = "1",
            help = "Maximum depth for following links"
        )]
        max_depth: usize,
    },
}

impl ToolCommands {
    /// Get the model name from the command
    pub fn model(&self) -> &str {
        match self {
            ToolCommands::Search { model, .. } => model,
            ToolCommands::Browse { model, .. } => model,
        }
    }
}
