# Kowalski CLI

A command-line interface for the Kowalski AI agent framework, providing easy access to all Kowalski capabilities through a unified CLI.

## Overview

The Kowalski CLI provides a comprehensive command-line interface for interacting with AI agents, managing models, and performing various AI-powered tasks. It serves as the main entry point for users who prefer command-line tools over programmatic APIs.

## Features

- **Multi-Agent Support**: Access to all Kowalski agents (academic, code, data, web)
- **Model Management**: List, check, and manage Ollama models
- **Conversation Interface**: Interactive chat with AI agents
- **File Processing**: Analyze documents, code, and data files
- **Web Research**: Perform web searches and scrape information
- **Academic Research**: Analyze research papers and academic content
- **Code Analysis**: Analyze, refactor, and generate code
- **Data Analysis**: Process and analyze data files (optional feature)

## Installation

### From Source
```bash
git clone https://github.com/yarenty/kowalski.git
cd kowalski
cargo install --path kowalski-cli
```

### From Crates.io (when published)
```bash
cargo install kowalski-cli
```

## Prerequisites

- **Ollama**: Make sure Ollama is installed and running
- **Rust**: Rust toolchain (for building from source)
- **Models**: Download the models you want to use (e.g., `ollama pull llama2`)

## Usage

### Basic Commands

```bash
# Show help
kowalski --help

# Show version
kowalski --version

# Show help for a specific command
kowalski chat --help
kowalski academic --help
kowalski model --help
```

### Model Management

```bash
# List available models
kowalski model list

# Check if a specific model exists
kowalski model check llama2

# Pull a model from Ollama
kowalski model pull llama2

# Show model information
kowalski model info llama2
```

### Chat Interface

```bash
# Start an interactive chat session
kowalski chat

# Chat with a specific model
kowalski chat --model llama2

# Chat with a specific agent type
kowalski chat --agent academic

# Chat with a file context
kowalski chat --file document.txt

# Chat with custom system prompt
kowalski chat --prompt "You are a helpful coding assistant"
```

### Academic Research

```bash
# Analyze a research paper
kowalski academic analyze paper.pdf

# Summarize academic content
kowalski academic summarize paper.pdf

# Extract key insights
kowalski academic insights paper.pdf

# Compare multiple papers
kowalski academic compare paper1.pdf paper2.pdf

# Generate research questions
kowalski academic questions paper.pdf
```

### Code Analysis

```bash
# Analyze a code file
kowalski code analyze src/main.rs

# Refactor code
kowalski code refactor src/main.rs

# Generate documentation
kowalski code docs src/main.rs

# Review code quality
kowalski code review src/main.rs

# Suggest improvements
kowalski code improve src/main.rs
```

### Web Research

```bash
# Search the web
kowalski web search "Rust async programming"

# Scrape a webpage
kowalski web scrape https://example.com

# Research a topic
kowalski web research "machine learning trends 2024"

# Extract information from multiple sources
kowalski web extract "climate change" --sources 5
```

### Data Analysis (Optional Feature)

```bash
# Analyze CSV data
kowalski data analyze data.csv

# Generate statistics
kowalski data stats data.csv

# Create visualizations
kowalski data visualize data.csv

# Detect patterns
kowalski data patterns data.csv

# Clean data
kowalski data clean data.csv
```

## Configuration

The CLI uses the same configuration system as the Kowalski framework. Configuration files are automatically loaded from:

1. `~/.config/kowalski/config.toml` (user config)
2. `./config.toml` (local config)
3. Environment variables

### Example Configuration

```toml
# ~/.config/kowalski/config.toml
[ollama]
host = "localhost"
port = 11434
model = "llama2"

[chat]
max_history = 100
enable_streaming = true
temperature = 0.7
max_tokens = 2048

[logging]
level = "info"
format = "json"
```

### Environment Variables

```bash
# Set Ollama host
export KOWALSKI_OLLAMA_HOST=localhost

# Set default model
export KOWALSKI_OLLAMA_MODEL=llama2

# Set log level
export KOWALSKI_LOG_LEVEL=debug
```

## Command Options

### Global Options

- `--config <FILE>`: Specify configuration file
- `--log-level <LEVEL>`: Set logging level (debug, info, warn, error)
- `--quiet`: Suppress output
- `--verbose`: Enable verbose output

### Model Options

- `--model <NAME>`: Specify model to use
- `--host <HOST>`: Ollama host address
- `--port <PORT>`: Ollama port number

### Output Options

- `--output <FILE>`: Write output to file
- `--format <FORMAT>`: Output format (text, json, yaml)
- `--pretty`: Pretty-print JSON output

## Examples

### Interactive Development Session

```bash
# Start a coding session
kowalski chat --agent code --model codellama --prompt "You are a Rust expert"

# Ask for help with async programming
> How do I handle errors in async Rust?

# Get code review
kowalski code review src/main.rs --model codellama

# Refactor based on suggestions
kowalski code refactor src/main.rs --suggestions
```

### Research Workflow

```bash
# Research a topic
kowalski web research "Rust async programming best practices"

# Download and analyze papers
kowalski academic analyze paper1.pdf paper2.pdf

# Generate summary
kowalski academic summarize --output summary.md

# Ask follow-up questions
kowalski chat --file summary.md --agent academic
```

### Data Analysis Pipeline

```bash
# Analyze dataset
kowalski data analyze dataset.csv

# Generate insights
kowalski data insights dataset.csv --output insights.json

# Create visualizations
kowalski data visualize dataset.csv --format png

# Generate report
kowalski data report dataset.csv --output report.md
```

## Error Handling

The CLI provides detailed error messages and suggestions:

```bash
# If Ollama is not running
Error: Failed to connect to Ollama at localhost:11434
Suggestion: Start Ollama with 'ollama serve'

# If model is not found
Error: Model 'llama2' not found
Suggestion: Pull the model with 'ollama pull llama2'

# If file is not found
Error: File 'nonexistent.pdf' not found
Suggestion: Check the file path and permissions
```

## Troubleshooting

### Common Issues

1. **Ollama Connection Failed**
   ```bash
   # Check if Ollama is running
   curl http://localhost:11434/api/tags
   
   # Start Ollama if needed
   ollama serve
   ```

2. **Model Not Found**
   ```bash
   # List available models
   ollama list
   
   # Pull missing model
   ollama pull llama2
   ```

3. **Permission Issues**
   ```bash
   # Check file permissions
   ls -la config.toml
   
   # Fix permissions if needed
   chmod 644 config.toml
   ```

4. **Memory Issues**
   ```bash
   # Use smaller models for limited memory
   ollama pull llama2:7b
   
   # Adjust model settings in config
   ```

### Getting Help

```bash
# General help
kowalski --help

# Command-specific help
kowalski chat --help
kowalski academic --help

# Debug mode
kowalski --log-level debug chat

# Check version and dependencies
kowalski --version
```

## Integration

The CLI integrates seamlessly with other Kowalski components:

- **Programmatic Use**: Use the CLI output in scripts
- **Pipeline Integration**: Chain commands with pipes
- **Automation**: Use in CI/CD pipelines
- **IDE Integration**: Use with VS Code, IntelliJ, etc.

## Contributing

Contributions are welcome! Please see the main Kowalski repository for contribution guidelines.

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Related

- [Kowalski Core](https://crates.io/crates/kowalski-core) - Core framework
- [Kowalski Tools](https://crates.io/crates/kowalski-tools) - Tool collection
- [Kowalski Agent Template](https://crates.io/crates/kowalski-agent-template) - Agent templates
- [Main Kowalski Repository](https://github.com/yarenty/kowalski) - Source code and documentation 