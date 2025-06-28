# Kowalski Tools

A comprehensive toolkit for the Kowalski AI agent framework, providing specialized tools for data processing, code analysis, web scraping, document processing, and more.

## Description

Kowalski Tools is a modular collection of specialized tools designed to extend the capabilities of the Kowalski AI agent framework. Each tool implements the `Tool` trait from `kowalski-core` and provides domain-specific functionality for various data processing and analysis tasks.

## Dependencies

### Core Dependencies
- **serde** (1.0) - Serialization/deserialization with derive features
- **serde_json** (1.0) - JSON serialization/deserialization
- **thiserror** (2.0) - Error handling utilities
- **anyhow** (1.0) - Error propagation utilities
- **async-trait** - Async trait support
- **tracing** - Logging and diagnostics
- **tokio** (1.32) - Async runtime with full features
- **chrono** - Date and time utilities
- **url** - URL parsing and manipulation

### Tool-Specific Dependencies
- **reqwest** (0.12) - HTTP client for web tools
- **scraper** (0.23) - HTML parsing and CSS selector support
- **lopdf** (0.36) - PDF processing library
- **csv** (1.1) - CSV parsing and processing

### Development Dependencies
- **mockall** (0.13) - Mocking framework for testing
- **wiremock** (0.6.0-rc.3) - HTTP mocking for integration tests

## Features

The module supports optional features that can be enabled:
- `web` - Web scraping and search functionality
- `pdf` - PDF document processing
- `data` - CSV and data analysis tools
- `code` - Code analysis tools

## Architecture

### Core Structure
```kowalski-tools/
├── src/
│   ├── lib.rs          # Main library entry point
│   ├── tool.rs         # Tool manager and utilities
│   ├── data.rs         # CSV and data processing tools
│   ├── code.rs         # Code analysis tools
│   ├── web/            # Web-related tools
│   │   ├── mod.rs
│   │   ├── search.rs   # Web search functionality
│   │   └── scrape.rs   # Web scraping functionality
│   └── document/       # Document processing tools
│       ├── mod.rs
│       └── pdf.rs      # PDF processing
```

### Design Patterns
- **Trait-based Architecture**: All tools implement the `Tool` trait from `kowalski-core`
- **Async Support**: Tools are designed for async execution using `async-trait`
- **Error Handling**: Comprehensive error handling using `KowalskiError` types
- **Parameter Validation**: Built-in parameter validation and type checking
- **Metadata Support**: Tools provide execution metadata for debugging and logging

### Tool Manager
The `ToolManager` provides centralized tool registration and execution:
- Tool registration and discovery
- Parameter validation
- Async execution handling
- Tool listing and metadata

## Tools

### CSV Tool
**Location**: `src/data.rs`  
**Tool Name**: `csv_tool`

A comprehensive CSV processing and analysis tool with statistical capabilities.

**Features**:
- Proper CSV parsing with error handling
- Statistical analysis (min, max, average, sum for numeric columns)
- Text analysis (unique counts, most common values)
- Configurable limits (max_rows, max_columns)
- Two task types: `process_csv` and `analyze_csv`

**Parameters**:
- `content` (required): CSV content to process
- `max_rows` (optional): Maximum number of rows to process
- `max_columns` (optional): Maximum number of columns to process

**Output**: JSON with headers, records, and statistical summary

### Code Analysis Tools
**Location**: `src/code.rs`

#### Java Analysis Tool
**Tool Name**: `java_analysis`

Comprehensive Java code analysis with metrics and quality suggestions.

**Features**:
- Code metrics (lines, characters, words, classes, methods, imports)
- Cyclomatic complexity calculation
- Code quality suggestions
- Basic syntax validation
- Complexity level assessment (Low/Medium/High)

**Parameters**:
- `content` (required): Java code to analyze

**Output**: JSON with metrics, complexity analysis, suggestions, and syntax errors

#### Python Analysis Tool
**Tool Name**: `python_analysis`

Python-specific code analysis with PEP 8 compliance checking.

**Features**:
- Python-specific metrics and analysis
- PEP 8 style guide compliance checking
- Function and class detection
- Import analysis
- Code quality suggestions

**Parameters**:
- `content` (required): Python code to analyze

**Output**: JSON with Python-specific metrics, PEP 8 violations, and suggestions

#### Rust Analysis Tool
**Tool Name**: `rust_analysis`

Rust code analysis with safety and performance considerations.

**Features**:
- Rust-specific syntax and safety analysis
- Ownership and borrowing pattern detection
- Performance optimization suggestions
- Error handling analysis
- Cargo.toml dependency analysis

**Parameters**:
- `content` (required): Rust code to analyze

**Output**: JSON with Rust-specific analysis, safety checks, and optimization suggestions

### Web Tools
**Location**: `src/web/`

#### Web Search Tool
**Tool Name**: `web_search`

Performs web searches using multiple search providers.

**Features**:
- Multiple search provider support (DuckDuckGo, Serper)
- Configurable result count
- Provider selection via parameters
- Environment-based API key configuration

**Parameters**:
- `query` (required): Search query string
- `num_results` (optional): Number of results (default: 3)
- `provider` (optional): Search provider (default: duckduckgo)

**Output**: JSON with search results and metadata

#### Web Scrape Tool
**Tool Name**: `web_scrape`

Scrapes web pages using CSS selectors with recursive link following.

**Features**:
- CSS selector-based content extraction
- Recursive link following with depth control
- HTML and text content extraction
- Error handling for network issues
- Configurable scraping depth

**Parameters**:
- `url` (required): URL to scrape
- `selectors` (required): Array of CSS selectors
- `follow_links` (optional): Follow links recursively (default: false)
- `max_depth` (optional): Maximum recursion depth (default: 1)

**Output**: JSON array of extracted content with metadata

### Document Tools
**Location**: `src/document/`

#### PDF Tool
**Tool Name**: `pdf_tool`

Comprehensive PDF processing with text, metadata, and image extraction.

**Features**:
- Text extraction from PDF pages
- Metadata extraction (title, author, creation date, etc.)
- Image extraction capabilities
- PDF structure analysis
- Error handling for corrupted files

**Parameters**:
- `file_path` (required): Path to PDF file
- `extract_text` (optional): Extract text content (default: true)
- `extract_metadata` (optional): Extract metadata (default: false)
- `extract_images` (optional): Extract images (default: false)

**Output**: JSON with extracted content based on selected options

## Usage Examples

### Basic Tool Usage
```rust
use kowalski_tools::{CsvTool, ToolManager};

let mut manager = ToolManager::new();
let csv_tool = CsvTool::new(1000, 50);
manager.register_tool(csv_tool);

// Execute CSV analysis
let input = ToolInput {
    task_type: "analyze_csv".to_string(),
    content: csv_content,
    parameters: HashMap::new(),
};

let result = manager.execute_tool("csv_tool", input).await?;
```

### Web Search Example
```rust
use kowalski_tools::web::WebSearchTool;

let search_tool = WebSearchTool::new("duckduckgo".to_string());
let input = ToolInput {
    task_type: "search".to_string(),
    content: "".to_string(),
    parameters: {
        let mut params = HashMap::new();
        params.insert("query".to_string(), json!("Rust programming"));
        params.insert("num_results".to_string(), json!(5));
        params
    },
};

let result = search_tool.execute(input).await?;
```

## Error Handling

All tools use the `KowalskiError` type for consistent error handling:
- `ToolExecution` - Errors during tool execution
- `ToolConfig` - Configuration errors
- `ContentProcessing` - Data processing errors
- `Execution` - General execution errors

## Testing

The module includes comprehensive tests for each tool:
- Unit tests for individual tool functionality
- Integration tests for tool interactions
- Mock-based testing for external dependencies
- HTTP mocking for web tool testing

## Future Enhancements

- Additional code analysis tools for more languages
- Enhanced PDF processing with OCR capabilities
- Database connectivity tools
- Machine learning model integration
- Real-time data streaming tools
- Advanced web scraping with JavaScript rendering