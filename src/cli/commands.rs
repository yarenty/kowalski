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
        /// Path to the research paper
        #[arg(short, long, help = "Path to the PDF or text file to analyze")]
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
            help = "Output format (text, json, or markdown)"
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
    /// Use AI-powered tools
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
}

/// Commands for managing AI models
#[derive(Subcommand, Debug)]
pub enum ModelCommands {
    /// List available models
    ///
    /// Shows all AI models that are currently installed and available for use.
    /// Displays information about each model including size and last modified date.
    List,
    /// Pull a new model
    ///
    /// Downloads and installs a new AI model. The model will be available for use
    /// after the download is complete.
    ///
    /// Example:
    ///   kowalski model pull llama2
    Pull {
        /// Name of the model to pull
        #[arg(help = "The name of the model to download")]
        name: String,
    },
    /// Remove a model
    ///
    /// Uninstalls an AI model to free up disk space. The model will need to be
    /// pulled again if you want to use it in the future.
    ///
    /// Example:
    ///   kowalski model remove llama2
    Remove {
        /// Name of the model to remove
        #[arg(help = "The name of the model to uninstall")]
        name: String,
    },
    /// Show model details
    ///
    /// Displays detailed information about a specific model, including its size,
    /// configuration, and capabilities.
    ///
    /// Example:
    ///   kowalski model show llama2
    Show {
        /// Name of the model to show
        #[arg(help = "The name of the model to display details for")]
        name: String,
    },
}

/// Commands for using AI-powered tools
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
        /// Your search query
        #[arg(help = "The search query to find information about")]
        query: String,
        /// Model to use
        #[arg(
            short,
            long,
            default_value = "llama2",
            help = "The AI model to use for processing results"
        )]
        model: String,
        /// Number of results to return
        #[arg(
            short,
            long,
            default_value = "5",
            help = "Maximum number of search results to process"
        )]
        limit: usize,
    },
    /// Scrape and analyze a webpage
    ///
    /// Downloads and analyzes the content of a webpage. Can optionally follow
    /// links to gather more information from related pages.
    ///
    /// Examples:
    ///   kowalski tool scrape "https://example.com"
    ///   kowalski tool scrape "https://docs.rs" --follow-links --max-depth 2
    Scrape {
        /// URL to scrape
        #[arg(help = "The URL of the webpage to analyze")]
        url: String,
        /// Model to use
        #[arg(
            short,
            long,
            default_value = "llama2",
            help = "The AI model to use for content analysis"
        )]
        model: String,
        /// Whether to follow links
        #[arg(
            short,
            long,
            default_value = "false",
            help = "Whether to follow and analyze linked pages"
        )]
        follow_links: bool,
        /// Maximum depth for link following
        #[arg(
            short,
            long,
            default_value = "1",
            help = "Maximum number of link hops to follow"
        )]
        max_depth: u32,
    },
    /// Analyze code
    ///
    /// Analyzes code files or directories to provide insights, suggestions, and
    /// answer questions about the code.
    ///
    /// Examples:
    ///   kowalski tool code ./src/main.rs
    ///   kowalski tool code ./src --language rust
    Code {
        /// Path to the code file or directory
        #[arg(short, long, help = "Path to the code file or directory to analyze")]
        path: PathBuf,
        /// Model to use
        #[arg(
            short,
            long,
            default_value = "llama2",
            help = "The AI model to use for code analysis"
        )]
        model: String,
        /// Language of the code
        #[arg(
            short,
            long,
            help = "Programming language of the code (optional, will be auto-detected if not specified)"
        )]
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
