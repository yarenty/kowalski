# kowalski-core AI Agent Documentation

> **READ THIS FIRST**: This file serves as the single source of truth for any AI agent (Claude, Gemini, Cursor, etc.) working on the `kowalski-core` component of the Kowalski repository. It aggregates architectural context, development workflows, and behavioral guidelines.

## Table of Contents
1. [Philosophy & Core Principles](#1-philosophy--core-principles)
2. [Project Identity](#2-project-identity)
3. [Architecture & Design Principles](#3-architecture--design-principles)
4. [Technology Stack](#4-technology-stack)
5. [Repository Structure](#5-repository-structure)
6. [Development Workflows](#6-development-workflows)
7. [Quality Standards](#7-quality-standards)
8. [Critical Rules & Protocols](#8-critical-rules--protocols)
9. [Implementation Status](#9-implementation-status)
10. [Common AI Tasks](#10-common-ai-tasks)

---

## 1. Philosophy & Core Principles

### Core Philosophy
- **Incremental progress over big bangs**: Break complex tasks into manageable stages
- **Learn from existing code**: Understand patterns before implementing new features
- **Clear intent over clever code**: Prioritize readability and maintainability
- **Simple over complex**: Keep implementations straightforward - prioritize solving problems over architectural complexity

### The Eight Honors and Eight Shames
| **Shame** | **Honor** |
|-----------|-----------|
| Guessing APIs | Careful research and documentation reading |
| Vague execution | Seeking confirmation before major changes |
| Assuming business logic | Human verification of requirements |
| Creating new interfaces | Reusing existing, proven patterns |
| Skipping validation | Proactive testing and error handling |
| Breaking architecture | Following established specifications |
| Pretending to understand | Honest acknowledgment of uncertainty |
| Blind modification | Careful, incremental refactoring |

### SOLID Principles Integration
Our codebase follows SOLID principles to ensure maintainable, scalable software.

**Quick Reference**: See [`tools/solid_principles_quick_reference.md`](tools/solid_principles_quick_reference.md) for essential patterns and checklists.

**Detailed Guide**: See [`tools/solid_principles_guide.md`](tools/solid_principles_guide.md) for comprehensive examples and implementation strategies.

#### Core SOLID Guidelines for AI Development
- **Single Responsibility (SRP)**: Before adding functionality, ask "Does this belong here?"
- **Open/Closed (OCP)**: Extend behavior through new classes/modules, not modifications
- **Liskov Substitution (LSP)**: Ensure any subclass can replace its parent without breaking functionality
- **Interface Segregation (ISP)**: Design small, specific interfaces rather than large, monolithic ones
- **Dependency Inversion (DIP)**: Inject dependencies rather than creating them directly

---

## 2. Project Identity

**Name**: kowalski-core  
**Purpose**: Core foundational abstractions, conversation logic, and agent traits.  
**Core Value Proposition**: Modular, extensible, and distributed architecture supporting standalone and federated deployments with privacy-preserving capabilities.  
**Primary Mechanism**: Multi-agent orchestration and pluggable tools interfacing with local (Ollama) and remote LLMs.  
**Target Users**: Kowalski framework agents and developers integrating kowalski-core.  

### Business Context
- **Problem Solved**: Complexity in building and managing secure, federated multi-agent LLM systems.
- **Success Metrics**: Extensibility of tools, performance of async operations, successful federated task execution.
- **Key Constraints**: Rust-based architecture, efficient execution, secure multi-party computation.

---

## 3. Architecture & Design Principles

### Architectural Patterns
- **Actor Model**: Agent abstractions and isolated execution contexts
- **Federated Architecture**: Multi-agent orchestration and secure computation
- **Pluggable Architecture**: Extensible toolchain and provider support

### Design Patterns in Use
- **Repository Pattern**: For data access abstraction
- **Factory Pattern**: For object creation
- **Strategy Pattern**: For algorithm selection
- **Observer Pattern**: For event handling
- **Message Passing Pattern**: For agent-to-agent communication
- **Plugin Pattern**: For dynamic tool integration

### Cross-Cutting Concerns
- **Logging**: Standard Rust tracing/logging
- **Error Handling**: Centralized error types (`KowalskiError`)
- **Security**: Secure multi-party computation (MPC) and role-based access
- **Performance**: Async-first using Tokio
- **Monitoring**: Built-in activity tracking and LLM observability

### Memory stack and dependencies (design)

**Qdrant** appeared in an **initial proof of concept** for vector-backed semantic memory. The **ongoing goal** is **simplicity, robustness, and minimal dependencies**: reduce **moving parts** (fewer required services), shrink the **failure surface**, and prefer **embedded / in-process** defaults. Details: [`docs/DESIGN_MEMORY_AND_DEPENDENCIES.md`](../docs/DESIGN_MEMORY_AND_DEPENDENCIES.md) and [`MEMORY_ARCHITECTURE.md`](./MEMORY_ARCHITECTURE.md).

---

## 4. Technology Stack

### Primary Technologies
- **Rust**
- **Tokio (Async Runtime)**
- **Ollama (Local LLMs)**
- **LLM Provider APIs (OpenAI, Anthropic, etc.)**

### Development Tools
- **Version Control**: Git
- **Build System**: Cargo
- **Testing Framework**: Cargo test
- **CI/CD**: GitHub Actions
- **Code Quality**: Clippy, rustfmt

### External Dependencies
- **Serde**: Serialization/Deserialization
- **Reqwest**: HTTP client
- **Tracing**: Logging and instrumentation
- **SQLx**: SQLite (always). **PostgreSQL** (`sqlx/postgres`) and **`pgvector`** (`pgvector/sqlx` SQLx integration) are enabled only by the **`postgres`** feature on `kowalski-core` (e.g. `--features postgres`, or `kowalski-cli --features postgres`, or `kowalski --features full`).

---

## 5. Repository Structure

### Directory Layout
```
kowalski/
├── kowalski-core/           # Core agent abstractions, conversation, roles, config, toolchain
├── kowalski-tools/          # Pluggable tools (code, data, web, document, etc.)
├── kowalski-agent-template/ # Agent builder, base agent, and templates
├── kowalski-federation/     # Multi-agent orchestration (WIP)
├── kowalski-cli/            # Command-line interface
├── kowalski-academic-agent/ # Academic research agent
├── kowalski-code-agent/     # Code analysis agent
├── kowalski-data-agent/     # Data analysis agent
└── kowalski-web-agent/      # Web research agent
```

### Component-Specific Documentation
**⚠️ CRITICAL**: Each major component contains its own `AGENTS.md` file with detailed information:

- [kowalski-core/AGENTS.md](kowalski-core/AGENTS.md)
- [kowalski-tools/AGENTS.md](kowalski-tools/AGENTS.md)
- [kowalski-agent-template/AGENTS.md](kowalski-agent-template/AGENTS.md)
- [kowalski-federation/AGENTS.md](kowalski-federation/AGENTS.md)
- [kowalski-cli/AGENTS.md](kowalski-cli/AGENTS.md)
- [kowalski-academic-agent/AGENTS.md](kowalski-academic-agent/AGENTS.md)
- [kowalski-code-agent/AGENTS.md](kowalski-code-agent/AGENTS.md)
- [kowalski-data-agent/AGENTS.md](kowalski-data-agent/AGENTS.md)
- [kowalski-web-agent/AGENTS.md](kowalski-web-agent/AGENTS.md)

**Rule**: Before making changes to any component, **always read its specific AGENTS.md first** to understand:
- Component architecture and responsibilities
- Development workflows and testing approaches  
- API patterns and integration points
- Common issues and troubleshooting steps
- Technology-specific considerations

### Service Architecture
- **Agent Workers**: Independent processing units
- **Federation Hub**: Orchestrates communication and tasks (WIP)
- **Tool Proxies**: Interfaces to external services and utilities

---

## 6. Development Workflows

### Initial Setup
1. Install Rust (`rustup`)
2. Install Ollama
3. Clone repo: `git clone https://github.com/yarenty/kowalski.git`
4. Build: `cargo build --release`

### Daily Development Workflow
1. **Start**: Review current task in `task.md`
2. **Plan**: Update task phases and current status
3. **Research**: Read relevant component documentation
4. **Implement**: Follow incremental development approach
5. **Test**: Validate changes incrementally
6. **Document**: Update task progress and decisions
7. **Review**: Ensure code meets quality standards

### Feature Development Process
1. **Analysis**: Understand requirements and constraints
2. **Design**: Plan implementation following SOLID principles
3. **Implementation**: Write code in small, testable increments
4. **Testing**: Unit tests, integration tests, and manual validation
5. **Documentation**: Update relevant docs and component guides
6. **Review**: Code review and architectural compliance check

### Build and Deployment
- **Build**: `cargo build --release`
- **Deployment**: Binaries built and run directly. Pluggable agents as independent processes.

### Testing Strategy
- **Unit Tests**: Cargo `#[test]` macros
- **Integration Tests**: Tests in `tests/` directory
- **End-to-End Tests**: Full CLI interactions
- **Performance Tests**: Criterion benchmarks where applicable

---

## 7. Quality Standards

### Code Quality
- **English Only**: All comments, documentation, and naming in English
- **Self-Documenting Code**: Clear naming conventions over extensive comments
- **No Unnecessary Comments**: Let clear code speak for itself
- **Consistent Style**: Follow established formatting and naming conventions

### Documentation Standards
- **API Documentation**: Rustdoc inline (`///`)
- **Architecture Decision Records**: Document significant architectural choices
- **Component Guides**: Maintain up-to-date component-specific documentation
- **Task Documentation**: Use structured task planning for complex work

### Testing Standards
- **Test Coverage**: Target > 80% for core logic
- **Test Naming**: Clear, descriptive test names that explain intent
- **Test Structure**: Arrange-Act-Assert pattern
- **Integration Testing**: Test component interactions

### Performance Standards
- **Response Time**: <100ms for local processing (excluding LLM generation time)
- **Throughput**: Configurable concurrent async actors
- **Resource Usage**: Optimized memory footprint, single binary per component
- **Scalability**: Horizontal scaling of agent processes

---

## 8. Critical Rules & Protocols

### Rule 0: Read Component Documentation First
**Before working on any specific component, ALWAYS read its AGENTS.md file first.**

Component-specific files contain crucial information about:
- Architecture patterns specific to that component
- Development workflows and testing procedures
- Technology-specific considerations and best practices
- Common issues and troubleshooting steps
- Integration patterns with other services

### Rule 1: Create Plan First
Never start a complex task without creating a `task.md` file. Use the template in `tools/task_template.md`.

**When to create a task plan:**
- Multi-step tasks (3+ steps)
- Research or analysis tasks
- Building/creating new components
- Tasks spanning multiple files or components

### Rule 2: The 2-Action Rule
> "After every 2 view/browser/search operations, IMMEDIATELY save key findings to text files."

This prevents loss of visual/multimodal information and maintains context across long sessions.

### Rule 3: Read Before You Decide
Before making major decisions, re-read the plan file and relevant documentation to ensure alignment with goals and architecture.

### Rule 4: Update After You Act
After completing any phase:
- Mark phase status: `pending` → `in_progress` → `complete`
- Log any errors encountered with resolution details
- Note files created, modified, or deleted
- Update decision log with rationale

### Rule 5: Log ALL Errors
Every error goes in the task plan file with:
- Error description
- Attempt number
- Resolution approach
- Lessons learned

```markdown
## Errors Encountered
| Error | Attempt | Resolution | Lessons Learned |
|-------|---------|------------|----------------|
| FileNotFoundError | 1 | Created default config | Check file existence first |
| API timeout | 2 | Added retry logic | Network calls need resilience |
```

### Rule 6: Never Repeat Failures
```
if action_failed:
    next_action != same_action
```
Track what you tried. Mutate the approach. Learn from failures.

### The 3-Strike Error Protocol

```
ATTEMPT 1: Diagnose & Fix
  → Read error message carefully
  → Identify root cause
  → Apply targeted fix

ATTEMPT 2: Alternative Approach  
  → Same error? Try different method
  → Different tool? Different library?
  → NEVER repeat exact same failing action

ATTEMPT 3: Broader Rethink
  → Question initial assumptions
  → Search for solutions and best practices
  → Consider updating the plan or approach

AFTER 3 FAILURES: Escalate to User
  → Explain what you tried in detail
  → Share the specific error messages
  → Ask for guidance or clarification
```

### Context Management Protocol

#### Read vs Write Decision Matrix
| Situation | Action | Reason |
|-----------|--------|--------|
| Just wrote a file | DON'T read | Content still in context |
| Viewed image/PDF | Write findings NOW | Multimodal data doesn't persist |
| Browser returned data | Write to file | Screenshots are temporary |
| Starting new phase | Read plan/findings | Re-orient if context is stale |
| Error occurred | Read relevant files | Need current state to debug |
| Resuming after gap | Read all planning files | Recover full state |

#### The 5-Question Context Check
If you can answer these questions, your context management is solid:

| Question | Answer Source |
|----------|---------------|
| Where am I? | Current phase in task.md |
| Where am I going? | Remaining phases in task.md |
| What's the goal? | Goal statement in plan |
| What have I learned? | Findings and decisions in task.md |
| What have I done? | Progress tracking in task.md |

---

## 9. Implementation Status

### Current Status
Active development. Core, Tools, and specialized agents (Academic, Code, Data, Web) are operational. Federation is WIP.

### Roadmap
See `ROADMAP.md` for latest features and future plans.

### Technical Debt
- Tools are currently monolithic, pending refactor into granular tool crates.
- Federation protocol finalization.

### Known Issues
- Ollama models must be running contextually.
- Multi-agent coordination overhead.

---

## Legacy Component Context

# kowalski-core: Core Agent Framework

## 1. Purpose

The `kowalski-core` crate serves as the foundational library for the entire Kowalski multi-agent framework. It provides the essential abstractions, traits, and common implementations for building AI agents, managing conversations, integrating tools, and handling a multi-tiered memory system. It aims to be the robust, extensible, and high-performance backbone upon which all specialized Kowalski agents are built.

## 2. Structure

The `kowalski-core/src` directory is well-organized into logical modules:

*   **`agent/`**: Defines the core `Agent` trait, which all specialized agents must implement, and the `BaseAgent` struct, providing default implementations for common agent functionalities (conversation management, memory integration, tool execution loop).
*   **`conversation/`**: Manages the structure and history of conversations, including messages and models used.
*   **`config.rs`**: Handles configuration loading and management for the core framework.
*   **`error.rs`**: Defines custom error types (`KowalskiError`) for consistent error handling across the framework.
*   **`logging/`**: Provides logging and tracing utilities.
*   **`memory/`**: Implements the sophisticated multi-tiered memory architecture (Working, Episodic, Semantic), including a `MemoryProvider` trait and specific implementations for each tier. It also includes `consolidation.rs` for memory "weaving."
*   **`model/`**: Contains the `ModelManager` for interacting with LLM providers (currently tightly coupled to Ollama).
*   **`role/`**: Defines concepts like `Role`, `Audience`, `Preset`, and `Style` to guide LLM behavior.
*   **`template/`**: Provides mechanisms for creating agents from templates, exemplified by `DefaultTemplate`.
*   **`tools.rs`**: Defines the `Tool` trait, `ToolInput`, `ToolOutput`, `ToolCall`, and `ToolParameter` structs, which are fundamental for tool integration.
*   **`tool_chain.rs`**: Intends to manage a chain of tools, but its current implementation is a basic tool registry.

## 3. Strengths

*   **Robust Core Abstractions:** The `Agent` and `Tool` traits are well-defined and provide a strong foundation for building extensible agent functionalities.
*   **Sophisticated Memory Architecture:** The multi-tiered memory system (Working, Episodic, Semantic) is a significant strength: in-memory working memory, **SQLite episodic** buffer (`episodic_kv`), and semantic storage via **in-process** vector similarity plus a **simple `HashMap` of relation triples** (Qdrant was an early PoC for vectors; `petgraph` removed—see [`docs/DESIGN_MEMORY_AND_DEPENDENCIES.md`](../docs/DESIGN_MEMORY_AND_DEPENDENCIES.md)). The hybrid retrieval and memory consolidation ("Memory Weaver") are advanced features.
*   **Modular Design:** The logical separation into distinct modules (`agent`, `memory`, `model`, `tools`, etc.) enhances maintainability and understanding.
*   **ReAct-style Tool Execution:** The `BaseAgent` includes a `chat_with_tools` method that implements a ReAct-style loop for LLM-driven tool use, which is a powerful pattern.
*   **Rust-native Performance and Safety:** Leveraging Rust ensures high performance, memory safety, and concurrency, crucial for an agentic framework.

## 4. Weaknesses

*   **Singleton Memory Providers:** The use of `tokio::sync::OnceCell` for `EpisodicBuffer` and `SemanticStore` creates singletons. This design choice severely limits the ability to:
    *   Run multiple independent agents within the same process without shared memory conflicts.
    *   Conduct isolated unit testing for memory components.
    *   Manage the lifecycle and configuration of memory providers dynamically.
    *   This is a critical architectural flaw for a multi-agent framework.
*   **Tool Management Inconsistencies:**
    *   The `tool_chain.rs` crate defines a `ToolChain` that is functionally a simple tool registry, not a chain, leading to misleading naming and limited capabilities.
    *   The `ToolManager` in `kowalski-tools` (which is a better design) is not integrated or used by `BaseAgent`, indicating a lack of centralized tool management.
    *   Hardcoded tool-chaining logic exists directly within `agent/mod.rs` (`chat_with_tools`), which is inflexible and duplicates concerns.
*   **LLM Provider Coupling:** The `ModelManager` is tightly coupled to Ollama, lacking a generic `ModelProvider` trait or similar abstraction to easily integrate other LLM services (e.g., OpenAI, Anthropic).
*   **JSON Parsing Fragility:** The logic for extracting JSON tool calls from raw LLM responses is somewhat brittle (simple `{` and `}` matching), which could lead to parsing failures with complex or malformed LLM outputs.
*   **Redundant Type Definitions:** Duplication of `ToolParameter` and `ParameterType` definitions exists between `kowalski-core` and `kowalski-tools`, which should be resolved to avoid confusion and maintain a single source of truth.

## 5. Potential Improvements & Integration into Rebuild

To align with a "total rebuild" aiming for "100x better than OpenClaw" while maintaining Kowalski's strengths, `kowalski-core` needs significant architectural refinement:

*   **Dependency Injection for Memory:** Refactor memory providers to use dependency injection (e.g., passing `Arc<dyn MemoryProvider>` or `Box<dyn MemoryProvider>`) to eliminate singletons, enable independent agent instances, and facilitate testing. This is the highest priority.
*   **Unified and Flexible Tool Management:**
    *   Develop a single, robust `ToolManager` (similar to the one in `kowalski-tools`) within `kowalski-core` that handles tool registration, discovery, validation, and execution.
    *   Introduce a flexible tool-chaining or "tool-orchestration" mechanism (e.g., using a directed acyclic graph or rule-based system) that is configurable and not hardcoded.
    *   Implement dynamic tool definition generation (e.g., OpenAPI-style JSON schema) that can be passed to LLMs, removing the need for manual prompt updates.
*   **LLM Provider Abstraction:** Introduce a `LLMProvider` trait (or similar) to abstract different LLM APIs, allowing for easy integration of OpenAI, Anthropic, Gemini, etc., alongside Ollama.
*   **Robust JSON Parsing:** Employ a more sophisticated JSON parsing library or strategy to reliably extract tool calls from LLM responses, even when embedded in natural language.
*   **Refactor `conversation` and `tool_chain`**: Review the commented out `conversation` module in `lib.rs` and the `ToolCall` ambiguity. Re-evaluate `tool_chain.rs`'s purpose; it might be better absorbed into a robust `ToolManager`.
*   **Consistent Error Handling:** Standardize the error handling strategy across all components.

These changes will make `kowalski-core` more robust, flexible, and scalable, laying a much stronger foundation for the advanced multi-agent and federation capabilities envisioned in the rebuild plan.

---

## 10. Common AI Tasks

### Code Review Checklist
- [ ] Follows SOLID principles (use [quick reference](tools/solid_principles_quick_reference.md))
- [ ] Maintains existing architectural patterns
- [ ] Includes appropriate tests
- [ ] Updates relevant documentation
- [ ] Handles errors gracefully
- [ ] Follows code quality standards

### Refactoring Guidelines
- [ ] Understand existing code thoroughly before changing
- [ ] Make small, incremental changes
- [ ] Maintain backward compatibility where possible
- [ ] Update tests to reflect changes
- [ ] Document architectural decisions

### New Feature Development
- [ ] Create task plan using template
- [ ] Research existing patterns and components
- [ ] Design following SOLID principles
- [ ] Implement incrementally with tests
- [ ] Update component documentation
- [ ] Perform integration testing

### Debugging Process
- [ ] Reproduce the issue consistently
- [ ] Identify root cause, not just symptoms
- [ ] Apply targeted fix following 3-strike protocol
- [ ] Add tests to prevent regression
- [ ] Document resolution in task plan

### Documentation Updates
- [ ] Keep component AGENTS.md files current
- [ ] Update API documentation for changes
- [ ] Record architectural decisions
- [ ] Maintain task planning discipline
- [ ] Update this master AGENTS.md as project evolves

---

## Anti-Patterns to Avoid

| ❌ Don't | ✅ Do Instead |
|----------|---------------|
| Use temporary notes for persistence | Create structured files (task.md, findings.md) |
| State goals once and forget | Re-read plans before major decisions |
| Hide errors and retry silently | Log all errors with resolution details |
| Stuff everything in context | Store large content in organized files |
| Start executing immediately | Create task plan FIRST |
| Repeat failed actions | Track attempts, mutate approach systematically |
| Violate SOLID principles for speed | Take time to design proper abstractions |
| Skip component documentation | Always read component AGENTS.md first |

---

**Remember**: This documentation evolves with the project. Keep it updated as architectural decisions are made and new patterns emerge. The goal is to enable efficient, high-quality AI-assisted development that maintains consistency and follows best practices.
