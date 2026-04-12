# kowalski AI Agent Documentation

> **READ THIS FIRST**: This file serves as the single source of truth for any AI agent (Claude, Gemini, Cursor, etc.) working on the `kowalski` facade component of the Kowalski repository. It aggregates architectural context, development workflows, and behavioral guidelines.

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

**Name**: kowalski  
**Purpose**: The main facade crate providing a unified API for the Kowalski framework.  
**Core Value Proposition**: Modular, extensible, and distributed architecture supporting standalone and federated deployments with privacy-preserving capabilities.  
**Primary Mechanism**: Multi-agent orchestration and pluggable tools interfacing with local (Ollama) and remote LLMs.  
**Target Users**: Developers integrating the Kowalski framework into their applications.  

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

**Qdrant** was used in an **initial PoC** for semantic memory. The project prioritizes a **simple, robust, dependency-light** stack and **fewer moving parts**; see [`../docs/DESIGN_MEMORY_AND_DEPENDENCIES.md`](../docs/DESIGN_MEMORY_AND_DEPENDENCIES.md).

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

# kowalski: Facade Crate for the Kowalski Framework

## 1. Purpose

The `kowalski` crate acts as a facade, re-exporting the functionality of other crates within the Kowalski workspace (e.g., `kowalski-core`, `kowalski-tools`, specialized agents, etc.). Its primary purpose is to provide a single, unified, and easy-to-use API for developers who want to integrate the Kowalski framework into their applications. By using feature flags, it allows users to compile only the necessary components, promoting a lean and customizable dependency.

## 2. Structure

The `kowalski` crate's structure is defined primarily by its `Cargo.toml` and `src/lib.rs` files:

*   **`Cargo.toml`**: Lists all other workspace crates as optional dependencies, controlled by feature flags (e.g., `academic`, `code`, `data`, `web`, `federation`, `cli`, `full`). It also includes common workspace-level dependencies.
*   **`src/lib.rs`**: Uses `pub use` statements and `#[cfg(feature = "...")]` directives to conditionally re-export public APIs from the dependent crates.

## 3. Strengths

*   **Unified API:** Provides a single entry point for developers, simplifying dependency management and integration. Users only need to add `kowalski` to their `Cargo.toml`.
*   **Modularity and Customization:** Feature flags enable users to selectively include only the parts of the framework they need, reducing build times and final binary sizes. This is excellent for flexibility.
*   **Clear Publishing Strategy:** The defined publishing order (core components first, then agents, then CLI, then facade) ensures that `crates.io` dependencies are correctly resolved.
*   **Good Design Pattern:** Utilizing a facade crate is a widely recognized best practice for managing complex, multi-crate Rust projects, offering a clean public interface while maintaining internal modularity.

## 4. Weaknesses

*   **Reliance on Sub-Crate Health:** The effectiveness of the `kowalski` facade is entirely dependent on the stability and API design of its underlying sub-crates. If sub-crates have breaking changes, the facade will also require updates.
*   **Abstraction Layer (Potential Complexity):** While simplifying the user's view, the facade adds another layer of abstraction. For developers needing to contribute to or deeply understand the framework, they still need to navigate the underlying crate structure.
*   **Potential for Feature Creep (in defaults):** If the `default` feature includes too many sub-crates, it might negate some of the benefits of modularity for users who only need a small subset of functionality. The current `default = ["academic", "code", "data", "web"]` is quite broad.
*   **`TemplateAgent` Abstraction Leak (Indirect):** As the facade re-exports components that internally rely on `TemplateAgent` (which is not public), this indirect abstraction leak remains a weakness for understanding and extending the framework.

## 5. Potential Improvements & Integration into Rebuild

To optimize the `kowalski` facade crate for the new rebuild:

*   **Streamlined Default Features:** Re-evaluate the `default` feature set. Consider making the default very minimal (e.g., only `kowalski-core` and a basic `BaseAgent` functionality) and requiring users to explicitly enable other features. This maximizes the benefit of modularity.
*   **Automated Feature Flag Management:** Develop tooling or conventions to ensure consistency between `Cargo.toml` feature flags and `src/lib.rs` conditional re-exports, minimizing manual errors.
*   **Clearer Module Organization in `lib.rs`:** As the number of re-exported modules grows, ensure that `src/lib.rs` maintains a clear and intuitive organization, perhaps grouping related re-exports.
*   **Documentation Focus:** Emphasize documentation for the `kowalski` crate's public API, clearly explaining how to enable and use its various features, as this will be the primary entry point for new users.
*   **Address Underlying Weaknesses First:** The facade's quality will inherently improve as the weaknesses of its constituent crates (e.g., singleton memory, inconsistent tool management, LLM provider coupling) are addressed. Prioritize fixing these at the `kowalski-core` level.
*   **Revisit `TemplateAgent` Exposure:** As discussed for specialized agents, resolve the `TemplateAgent` abstraction leak. If it's a foundational building block, make it public through the facade; otherwise, ensure specialized agents compose `BaseAgent` directly for clarity.

By focusing on these improvements, the `kowalski` facade can effectively serve as the user-friendly gateway to a powerful, flexible, and high-performance Kowalski framework.

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
