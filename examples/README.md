# Tool Examples

This directory contains examples demonstrating different functionalities of the agent system.

## Core Functionality Examples

### 1. Model Manager Example
Demonstrates model management capabilities:
```bash
cargo run --example model_manager_example
```

Features:
- Lists available models
- Checks model existence
- Downloads models if needed
- Shows download progress

### 2. Academic Agent Example
Shows how to process academic papers:
```bash
cargo run --example academic_agent_example
```

Features:
- PDF processing
- Scientific paper analysis
- Question generation
- Interactive follow-up queries

### 3. Tooling Agent Example
Demonstrates the complete tooling agent workflow:
```bash
cargo run --example tooling_agent_example
```

Features:
- Web searching
- Content processing
- Summary generation
- Conversation management

## Tool-Specific Examples

### 4. Search Example
Demonstrates the search functionality:
```bash
cargo run --example search_example
```

Features:
- Uses DuckDuckGo as the search provider
- Displays formatted search results
- Shows titles, URLs, and snippets

### 5. Dynamic Content Example
Shows how to handle JavaScript-heavy websites:
```bash
cargo run --example dynamic_content
```

Features:
- Processes social media pages
- Extracts content from dynamic sites
- Handles JavaScript-rendered content
- Collects metadata

### 6. Static Content Example
Demonstrates static website scraping:
```bash
cargo run --example static_content
```

Features:
- Processes multiple URLs in parallel
- Extracts structured content
- Collects metadata
- Shows content previews

## Configuration

Before running the examples, make sure you have a valid `config.toml` file with:

```toml
[ollama]
base_url = "http://localhost:11434"

[search]
provider = "duckduckgo"
api_key = "" # Optional

[chat]
temperature = 0.7
max_tokens = 2048
```

## Requirements

- Rust 1.75 or later
- Running Ollama instance
- Internet connection
- `env_logger` for logging (set `RUST_LOG=debug` for verbose output)

## Running Examples

To run any example with debug logging:
```bash
RUST_LOG=debug cargo run --example example_name
```

For example:
```bash
RUST_LOG=debug cargo run --example model_manager_example
``` 