# Kowalski Roadmap & Features (0.5.0+)

> "The future is modular, and so is Kowalski. Want a feature? Open an issue or submit a PR!"

## 🧩 Modular Architecture (since 0.5.0)

Kowalski is now split into clear, focused modules:
- **Core**: Foundational types, agent abstractions, conversation, roles, configuration, error handling, toolchain logic
- **Tools**: Pluggable tools for code, data, web, and document analysis
- **Template**: Agent builder, base agent, and ready-to-use agent templates
- **Federation**: (WIP) Multi-agent orchestration, registry, and protocols
- **Agents**: Specific agents (academic, code, data, web, etc.)
- **CLI**: Command-line interface

---

## Core
- [x] Agent abstraction & base agent
- [x] Conversation and memory management
- [x] Role, audience, preset, and style system
- [x] Tool and toolchain system
- [x] Unified error handling
- [x] Extensible configuration
- [ ] Long-term conversation storage (planned)
- [ ] Conversation search and indexing (planned)
- [ ] Context window management (planned)

## Tools
- [x] CSV/data analysis tool
- [x] Code analysis tools (Java, Python, Rust)
- [x] Web search tool (DuckDuckGo, Serper)
- [x] Web scraping tool (CSS selectors, recursive)
- [x] PDF/document processing tool
- [ ] Support for more document formats (DOCX, EPUB, HTML) (planned)
- [ ] Image processing and OCR (planned)
- [ ] Table extraction and processing (planned)
- [ ] Academic paper processing (citations, figures, LaTeX) (planned)

## Template
- [x] TemplateAgent abstraction
- [x] AgentBuilder for ergonomic construction
- [x] General-purpose agent template
- [x] Research agent template
- [ ] More templates for specific domains (planned)
- [ ] Dynamic tool loading/plugins (planned)

## Federation (Experimental)
- [x] Agent registry and membership
- [x] Role assignment (coordinator, worker, observer)
- [x] Task delegation and assignment
- [x] Message passing and broadcasting
- [ ] Protocol selection (A2A, ACP, MCP, or custom) (open)
- [ ] Secure agent authentication (planned)
- [ ] Persistent registry and orchestrator state (planned)
- [ ] Federation-wide logging and monitoring (planned)

## Agents
- [x] Academic agent
- [x] Code agent
- [x] Data agent
- [x] Web agent
- [ ] More specialized agents (planned)
- [ ] Agent templates for customer support, automation, etc. (planned)

## User Interface & Integration
- [x] CLI interface with rich formatting
- [ ] Web interface (planned)
- [ ] REST API (planned)
- [ ] WebSocket support (planned)
- [ ] Export conversations (PDF, HTML, Markdown) (planned)
- [ ] Slack/Discord/Teams integration (planned)
- [ ] Git/CI/CD integration (planned)

## Security & Privacy
- [ ] End-to-end encryption (planned)
- [ ] Role-based access control (planned)
- [ ] Conversation anonymization (planned)
- [ ] Audit logging (planned)
- [ ] Content filtering (planned)

## Analytics & Monitoring
- [ ] Usage statistics and analytics (planned)
- [ ] Performance monitoring (planned)
- [ ] Cost tracking (planned)
- [ ] Response quality metrics (planned)
- [ ] Error analytics and reporting (planned)

## Advanced Features
- [ ] Multi-language support (planned)
- [ ] Custom prompt templates (planned)
- [ ] Chain-of-thought visualization (planned)
- [ ] Semantic search across conversations (planned)
- [ ] Auto-summarization of long conversations (planned)

## Developer Tools
- [x] Plugin system (basic, planned for expansion)
- [ ] Custom model training tools (planned)
- [x] Debug mode with detailed logging
- [ ] Testing utilities (planned)
- [x] Documentation generator

---

## Long-Term Vision & Open Questions
- **Protocol selection for federation:** A2A, ACP, MCP, or custom?
- **Advanced agent orchestration:** Multi-agent, federated, and plugin-based development
- **Persistent state:** Should agent state and task history be persisted?
- **Security:** How do agents authenticate and authorize each other?
- **Scalability:** What are the bottlenecks for large federations?
- **Extensibility:** How can new agent types and protocols be plugged in?
- **Community:** Encourage contributions, feature requests, and protocol discussions

---

**Legend:**
- [x] Implemented
- [ ] Planned / In Progress / Experimental

> "The future is modular, and so is Kowalski. Want a feature? Open an issue or submit a PR!"