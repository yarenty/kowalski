# ROADMAP

## Phase 0: First Blood

focus on managable but fully functional agent that could give obvious benefits: 
- [x] connecting to local ollama server
- [x] processing user request - respond in streaming manner
- [x] simple roles
- [x] Rust interface
- [x] initial release 


## Current State Analysis
Based on the repository examination, Kowalski currently has:
- Basic agent implementations (GeneralAgent, AcademicAgent, ToolingAgent)
- CLI interface with chat, academic, model, and tool commands
- Ollama integration for LLM communication
- PDF processing and web search capabilities
- Role-based interactions system


# Kowalski Agent Framework Restructure Plan


> "Tools are like friends - they help you when you need them, but sometimes they crash when you need them most." - A Toolsmith
> "The internet is like a library where all the books are scattered on the floor." - A Web Crawler


## Overview
Transform Kowalski into a modular, extensible AI agent framework with clear separation between core functionality, templates, tools, and specialized agents.



## Target Architecture

```
kowalski/
├── kowalski-core/           # Core agent functionality
├── kowalski-templates/      # Agent templates and builders  
├── kowalski-tools/          # Standalone tool modules
├── kowalski-agents/         # Specialized agent implementations
├── kowalski-cli/            # Command-line interface
├── kowalski-federation/     # Multi-agent coordination (future)
└── examples/                # Usage examples and demos
```

## Phase 1: Core Foundation (Week 1-2)

### 1.1 Create `kowalski-core` crate
**Goal**: Establish the fundamental agent abstraction and communication layer.

**Structure**:
```rust
kowalski-core/
├── src/
│   ├── lib.rs
│   ├── agent/
│   │   ├── mod.rs
│   │   ├── base.rs          # BaseAgent trait
│   │   ├── conversation.rs  # Conversation management
│   │   └── response.rs      # Response handling
│   ├── llm/
│   │   ├── mod.rs
│   │   ├── client.rs        # LLM client abstraction
│   │   ├── ollama.rs        # Ollama implementation
│   │   └── types.rs         # Common LLM types
│   ├── config/
│   │   ├── mod.rs
│   │   └── settings.rs      # Configuration management
│   └── error.rs             # Error types
└── Cargo.toml
```

**Key Components**:
- `BaseAgent` trait with essential methods:
  ```rust
  #[async_trait]
  pub trait BaseAgent {
      async fn process(&self, input: &str, context: Option<Context>) -> Result<Response>;
      async fn start_conversation(&self, model: &str) -> ConversationId;
      fn with_tools(&mut self, tools: Vec<Box<dyn Tool>>) -> &mut Self;
      fn with_system_prompt(&mut self, prompt: &str) -> &mut Self;
  }
  ```
- Generic LLM client abstraction (not tied to Ollama)
- Conversation state management
- Configuration system with TOML/ENV support

**Technologies**:
- `async-trait` for async traits
- `serde` for configuration
- `tokio` for async runtime
- `thiserror` for error handling

**PoC Deliverable**: Basic agent that can communicate with Ollama and maintain conversation state.

### 1.2 Migrate existing functionality
- Extract current agent logic into core
- Preserve existing API compatibility
- Add comprehensive tests

## Phase 2: Tool Module System (Week 2-3)

### 2.1 Create `kowalski-tools` crate
**Goal**: Modular, reusable tools that any agent can use.

**Structure**:
```rust
kowalski-tools/
├── src/
│   ├── lib.rs
│   ├── tool.rs              # Tool trait definition
│   ├── web/
│   │   ├── mod.rs
│   │   ├── search.rs        # Web search tool
│   │   └── scraper.rs       # Web scraping tool
│   ├── document/
│   │   ├── mod.rs
│   │   ├── pdf.rs           # PDF processing
│   │   └── text.rs          # Text file processing
│   ├── data/
│   │   ├── mod.rs
│   │   ├── datafusion.rs    # DataFusion integration
│   │   └── csv.rs           # CSV processing
│   └── code/
│       ├── mod.rs
│       ├── analyzer.rs      # Code analysis
│       └── executor.rs      # Code execution (sandboxed)
└── Cargo.toml
```

**Tool Trait**:
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput>;
    fn parameters(&self) -> Vec<ToolParameter>;
}
```

**Key Tools to implement**:
1. **WebSearchTool**: Search engine integration (DuckDuckGo, Serper API)
2. **WebScrapeTool**: Web page content extraction
3. **PdfTool**: PDF text extraction and processing
4. **DataFusionTool**: SQL query execution on data files
5. **CodeAnalyzerTool**: Code analysis and explanation

**Technologies**:
- `reqwest` for HTTP requests
- `scraper` for HTML parsing
- `lopdf` or `pdf-reader` for PDF processing
- `datafusion` for SQL query engine
- `tree-sitter` for code analysis

**PoC Deliverable**: Web search and PDF processing tools working independently.

## Phase 3: Agent Templates (Week 3-4)

### 3.1 Create `kowalski-templates` crate
**Goal**: Template system for creating specialized agents quickly.

**Structure**:
```rust
kowalski-templates/
├── src/
│   ├── lib.rs
│   ├── builder.rs           # AgentBuilder
│   ├── templates/
│   │   ├── mod.rs
│   │   ├── research.rs      # Research agent template
│   │   ├── analysis.rs      # Data analysis template
│   │   └── general.rs       # General purpose template
│   └── prompts/
│       ├── mod.rs
│       └── library.rs       # Prompt template library
└── Cargo.toml
```

**AgentBuilder Pattern**:
```rust
pub struct AgentBuilder {
    core: BaseAgent,
    tools: Vec<Box<dyn Tool>>,
    system_prompt: String,
    temperature: f32,
}

impl AgentBuilder {
    pub fn new() -> Self { /* ... */ }
    pub fn with_template(template: AgentTemplate) -> Self { /* ... */ }
    pub fn add_tool<T: Tool + 'static>(mut self, tool: T) -> Self { /* ... */ }
    pub fn with_system_prompt(mut self, prompt: &str) -> Self { /* ... */ }
    pub fn build(self) -> Result<ConfiguredAgent> { /* ... */ }
}
```

**PoC Deliverable**: Agent builder that can create research and general-purpose agents using templates.

## Phase 4: Specialized Agents (Week 4-5)

### 4.1 Create `kowalski-agents` crate
**Goal**: Pre-built, optimized agents for specific use cases.

**Structure**:
```rust
kowalski-agents/
├── src/
│   ├── lib.rs
│   ├── web_search/
│   │   ├── mod.rs
│   │   └── agent.rs         # WebSearchAgent
│   ├── academic/
│   │   ├── mod.rs
│   │   └── agent.rs         # AcademicResearchAgent
│   ├── data_investigator/
│   │   ├── mod.rs
│   │   └── agent.rs         # DataInvestigatorAgent
│   └── code_assistant/
│       ├── mod.rs
│       └── agent.rs         # CodeAssistantAgent
└── Cargo.toml
```

**Agent Implementations**:

1. **WebSearchAgent**:
   - Tools: WebSearchTool, WebScrapeTool
   - Specialized for web research and information gathering
   - Custom prompts for synthesizing search results

2. **AcademicResearchAgent**:
   - Tools: PdfTool, WebSearchTool, CitationTool
   - Specialized for academic paper analysis
   - Citation formatting and reference management

3. **DataInvestigatorAgent**:
   - Tools: DataFusionTool, CsvTool, VisualizationTool
   - Specialized for data analysis and insights
   - SQL generation and data exploration

4. **CodeAssistantAgent**:
   - Tools: CodeAnalyzerTool, DocumentationTool
   - Specialized for code review and assistance
   - Multi-language support

**PoC Deliverable**: One fully functional specialized agent (WebSearchAgent) with proper tool integration.

## Phase 5: Enhanced CLI (Week 5-6)

### 5.1 Redesign `kowalski-cli`
**Goal**: Intuitive CLI that exposes all functionality and supports agent federation.

**New CLI Structure**:
```bash
# Core agent usage
kowalski chat "Hello world"                    # Generic agent
kowalski chat --template research "Find papers on rust async"

# Specialized agents
kowalski search "rust async programming"       # WebSearchAgent
kowalski academic --file paper.pdf            # AcademicAgent  
kowalski data --file data.csv "show trends"   # DataInvestigator
kowalski code --file src/ "review this code"  # CodeAssistant

# Tool usage (independent)
kowalski tool web-search "rust programming"
kowalski tool pdf-extract paper.pdf
kowalski tool data-query --file data.csv "SELECT * FROM table"

# Agent management
kowalski agent list                           # List available agents
kowalski agent create --template research my-agent  # Create custom agent
kowalski agent run my-agent "research task"         # Run custom agent

# Future: Federation
kowalski federation start                     # Start agent coordinator
kowalski federation add web-search academic  # Add agents to federation
kowalski federation run "complex multi-step task"
```

**Technologies**:
- `clap` v4 for CLI parsing with derive macros
- `dialoguer` for interactive prompts
- `indicatif` for progress bars
- `console` for colored output

**PoC Deliverable**: CLI that can create and run agents using templates and tools.

## Phase 6: Multi-Agent Federation (Week 6-7)

### 6.1 Create `kowalski-federation` crate (Future Phase)
**Goal**: Coordinate multiple agents working together on complex tasks.

**Core Concepts**:
- Task decomposition and delegation
- Agent communication protocols
- Result aggregation and synthesis
- Workflow orchestration

## Implementation Priorities for Fast PoC

### Week 1: Core + Basic Tool
1. Create `kowalski-core` with BaseAgent trait
2. Implement basic Ollama integration
3. Create WebSearchTool in `kowalski-tools`
4. Simple CLI to test core + web search

### Week 2: Template System
1. Create `kowalski-templates` with AgentBuilder
2. Implement research agent template
3. CLI commands for template-based agents
4. Documentation and examples

### Week 3: First Specialized Agent
1. Complete WebSearchAgent in `kowalski-agents`
2. Enhanced CLI for specialized agents
3. Integration tests
4. Performance optimization

### Week 4: Data Tools + Academic Agent
1. Implement DataFusion integration
2. Complete AcademicResearchAgent
3. PDF processing tools
4. End-to-end examples

## Technology Stack Recommendations

### Core Dependencies
- **Async Runtime**: `tokio` (already using)
- **HTTP Client**: `reqwest` with JSON support
- **Serialization**: `serde` with derive macros
- **Error Handling**: `thiserror` + `anyhow`
- **Configuration**: `config` crate with TOML support
- **Logging**: `tracing` + `tracing-subscriber`

### Tool-Specific Dependencies
- **Web Scraping**: `scraper` + `html5ever`
- **PDF Processing**: `lopdf` or `pdf-reader`
- **Data Processing**: `datafusion` + `arrow`
- **Code Analysis**: `tree-sitter` + language grammars
- **CLI**: `clap` v4 with derive features

### Testing & Quality
- **Testing**: `tokio-test` for async tests
- **Mocking**: `mockall` for tool mocking
- **Benchmarking**: `criterion` for performance tests
- **Documentation**: `cargo-doc` with examples

## Migration Strategy

1. **Preserve Compatibility**: Keep existing APIs working during transition
2. **Gradual Migration**: Move functionality module by module
3. **Feature Flags**: Use Cargo features to enable/disable components
4. **Documentation**: Update docs with each phase
5. **Testing**: Maintain test coverage throughout restructure

## Missing Components to Add

1. **Error Recovery**: Robust error handling and retry mechanisms
2. **Caching**: Tool result caching for performance
3. **Rate Limiting**: API rate limiting for external services
4. **Authentication**: API key management for external services
5. **Metrics**: Usage tracking and performance metrics
6. **Plugin System**: Dynamic tool loading
7. **Configuration UI**: Web interface for agent configuration
8. **Persistence**: Conversation and result storage
9. **Security**: Input validation and sandboxing
10. **Documentation**: Comprehensive user guides and API docs

## Success Metrics

### Phase 1 Success:
- Core agent can handle basic conversations
- Clean separation between core and implementation
- 90%+ test coverage for core functionality

### Phase 2 Success:  
- At least 3 tools working independently
- Tool system is extensible and well-documented
- Performance benchmarks established

### Phase 3 Success:
- Agent builder can create functional agents
- Templates reduce agent creation time by 80%
- Clear documentation for custom templates

### Phase 4 Success:
- One specialized agent outperforms generic agent in its domain
- Agent can use multiple tools effectively
- Real-world use case demonstrations

### Phase 5 Success:
- CLI is intuitive for new users
- All functionality accessible via CLI
- Interactive mode works smoothly

This plan prioritizes getting working functionality at each phase while building toward the complete vision. Each phase delivers value and can be used independently, ensuring you have a working product throughout the development process.

> "Strategy is like a GPS - it tells you where to go, but not how to avoid traffic." - A Project Manager