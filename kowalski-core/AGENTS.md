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

**Quick Reference**: See [`tools/solid_principles_quick_reference.md`](../tools/solid_principles_quick_reference.md) for essential patterns and checklists.

**Detailed Guide**: See [`tools/solid_principles_guide.md`](../tools/solid_principles_guide.md) for comprehensive examples and implementation strategies.

#### Core SOLID Guidelines for AI Development
- **Single Responsibility (SRP)**: Before adding functionality, ask "Does this belong here?"
- **Open/Closed (OCP)**: Extend behavior through new classes/modules, not modifications
- **Liskov Substitution (LSP)**: Ensure any subclass can replace its parent without breaking functionality
- **Interface Segregation (ISP)**: Design small, specific interfaces rather than large, monolithic ones
- **Dependency Inversion (DIP)**: Inject dependencies rather than creating them directly

---

## 2. Project Identity

**Name**: kowalski-core  
**Release**: **1.5.0** (see crate `Cargo.toml`).  
**Purpose**: Core foundational abstractions, conversation logic, agent traits, LLM providers, memory tiers, MCP client/hub, federation types, optional Postgres (pgvector) and graph helpers (AGE Cypher).  
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
kowalski/                         # repository root (you are in kowalski-core/)
├── kowalski-core/                # This crate: TemplateAgent, tools, memory, MCP, federation
├── kowalski-cli/                 # REPL, operators, extension, agent-app
├── kowalski/                     # Facade + HTTP server binary
├── kowalski-mcp-base/            # Shared MCP framework (transport + framing + rmcp serve)
├── kowalski-mcp-datafusion/      # Optional MCP server (DataFusion)
├── kowalski-mcp-rookery/         # Optional MCP server (Rookery horde builder)
├── ui/, examples/, docs/, tools/, resources/   # SQL migrations: `migrations/` within this crate
```

Tools and federation types live **in this crate** (`src/tools`, `src/tools/internal/`, `src/federation`, …)—not in separate `kowalski-tools` / `kowalski-federation` packages.

### Component-Specific Documentation
**⚠️ CRITICAL**: Read the `AGENTS.md` for the component you change:

- [Root AGENTS.md](../AGENTS.md)
- [kowalski-core/AGENTS.md](./AGENTS.md) (this crate)
- [kowalski-cli/AGENTS.md](../kowalski-cli/AGENTS.md)
- [kowalski/AGENTS.md](../kowalski/AGENTS.md)
- [kowalski-mcp-base/AGENTS.md](../kowalski-mcp-base/AGENTS.md)
- [kowalski-mcp-datafusion/AGENTS.md](../kowalski-mcp-datafusion/AGENTS.md)
- [kowalski-mcp-rookery/AGENTS.md](../kowalski-mcp-rookery/AGENTS.md)
- [ui/AGENTS.md](../ui/AGENTS.md)

**Rule**: Before making changes to any component, **always read its specific AGENTS.md first** to understand:
- Component architecture and responsibilities
- Development workflows and testing approaches  
- API patterns and integration points
- Common issues and troubleshooting steps
- Technology-specific considerations

### Service Architecture
- **`TemplateAgent`**, tools, memory, MCP client/hub, and federation primitives are implemented **here**.
- **HTTP** and CLI binaries live in other crates; this library is the core dependency.

### Horde-run store (`src/db/run_store.rs`)

Persisted run/step state machine for durable horde runs (the orchestrator in the
`kowalski` crate writes through it and resumes interrupted runs on startup). Typed
`RunStatus` (`pending|running|awaiting_input|done|error|cancelled`)
and `StepStatus` (`… delegating … succeeded|failed|skipped`), `manifest_snapshot` captured at
run start, per-step `attempt`, compact JSON `events` log, and `incomplete_runs()` for restart
scans. Resume support: `origin` (`RUN_ORIGIN_OPERATOR` default; non-operator origins may
auto-resume) and `resume_count` with atomic `increment_resume_count()` (attempt-cap guard
rail). SQLite-backed and zero-config: `RunStore::open_default(state_dir)` creates
`runs.sqlite` under the server state dir (`KOWALSKI_RUN_DB` env URL overrides —
name owned by `config::RUN_DB_ENV`, Rule 8). Schema: `migrations/sqlite/003_horde_runs.sql`
+ `004_run_resume.sql` (+ `migrations/postgres/005`/`006` parity). The embedded SQLite
migrator is shared (`db::SQLITE_MIGRATOR`) — memory subsystem and run store use one schema
lineage.

### Step handlers (`src/horde_step.rs`)

In-process execution for horde step kinds: `StepHandler` trait (`kind()` +
`async execute(&StepContext) -> StepOutcome`), `StepHandlerRegistry` (kind → handler,
built at server startup), and built-in deterministic handlers wrapping `horde_stages.rs`
and `source_bundle.rs`: **`verify`** (shell command in the operator project, markdown
artifact with `status:` frontmatter), **`apply`** (patch dry-run / env-gated execute of
```diff blocks), **`ingest`** (source capture under `workdir/debug/`). `StepOutcome`
reuses `StageStatus` (`pass`/`fail`) so conditional edges route unchanged. `StepContext`
carries run/step ids, the step's spec slice (`StepSpec`), workdir/horde root, prior
artifact, an event sink (published as `AgentMessage` by the orchestrator), LLM/tool
handles (unused by deterministic kinds), and a `CancellationToken` slot.

**Adding a step kind is one trait impl + one registry line:**

```rust
struct MyHandler;
#[async_trait]
impl StepHandler for MyHandler {
    fn kind(&self) -> &'static str { "my-kind" }
    async fn execute(&self, ctx: &StepContext<'_>) -> Result<StepOutcome, StepError> { /* … */ }
}
registry.register(Arc::new(MyHandler));
```

**`LlmStepHandler`** covers the LLM kinds (`LLM_STEP_KINDS`: `process`/`step`/`deliver`/
`final`, `compile`, `ask`, `lint`): prompt assembly (`build_llm_stage_request` — prompt
file + `@artifact@`/`@step:` context attachments + operator question for `ask`), then a
direct call into a shared `TemplateAgent` (tool allowlist + sandbox policy preserved,
memory off) — no HTTP self-loopback, no worker process. Kinds not in the registry are
delegated to federation workers; the orchestrator side (timeouts, cancellation,
TaskFinished feedback, process isolation) lives in the `kowalski` crate.

**Out-of-process executors share one entry point.** `IsolatedStepRequest` /
`IsolatedStepResponse` are the wire contract for running one step outside the caller's
process, and `execute_isolated_request(req, registry, events)` rebuilds the
`StepContext` and dispatches through the same registry — so the semantics of the
`agent-app exec-step` child (steps with `isolation = "process"`) and the SSE federation
worker can never drift from the in-process path. `IsolatedStepEvent` is the child's
stdout line protocol (zero or more `message` lines, then one `outcome`). Isolation
vocabulary (`ISOLATION_IN_PROCESS`/`ISOLATION_PROCESS`, `is_valid_isolation`) is owned
here.

### LLM providers (`src/llm/`)

`LLMProvider` (`provider.rs`) is the backend abstraction: `chat`, `embed`, `chat_stream`,
and — since native tool calling landed — `chat_with_tool_defs` + `supports_native_tools`.
Implementations: `OllamaProvider` (`ollama.rs`, raw `/api/chat` JSON) and `OpenAIProvider`
(`openai.rs`, async-openai — works against any OpenAI-compatible Chat Completions server).
`create_llm_provider(config)` (`mod.rs`) is the only factory; both binaries and step
handlers get providers through it.

**Native tool calling (opt-in, `[llm] native_tools = true`):**

- `ToolDefinition` (`provider.rs`) is the wire-format tool declaration and the **single
  owner** of the `Tool` metadata → JSON Schema conversion (`ToolDefinition::from_tool`);
  providers map it onto their client types (`wire_json()` for Ollama, typed
  `ChatCompletionTools` for OpenAI).
- `chat_with_tool_defs(model, messages, tools) -> ChatOutcome` sends declarations on the
  wire; `ChatOutcome` is either `Text` or structured `ToolCalls` (ids synthesized as
  `call_<n>` for Ollama, which sends none). The **caller** executes tools and sends results
  back as `role = "tool"` messages — `Message::tool_result(tool_call_id, content)`;
  `Message::assistant_tool_calls` records the requesting turn in history.
- `supports_native_tools(model)` reports the deployment's opt-in (default `false`); the
  default `chat_with_tool_defs` ignores `tools` and behaves like `chat`, so providers
  without tool support are unaffected. The ReAct JSON-in-text loop (`agent/mod.rs`
  `chat_with_tools`) remains the fallback and is unchanged.
- **The agent loop prefers the native path.** `[llm] tool_calling = "auto" | "native" |
  "react"` (default `auto`) picks the loop: `auto` runs native when
  `supports_native_tools` says the model is capable, `native`/`react` force one path.
  `BaseAgent::chat_with_tools_native` owns the native loop (structured calls executed in
  order, results fed back as tool-role messages, identical-consecutive-calls breaker,
  `MAX_TOOL_ITERATIONS` cap shared with ReAct); every `chat_with_tools*` entry point —
  the `Agent` trait method (REPL), `chat_with_tools_with_policy` (`/api/chat`, horde LLM
  steps), and `chat_with_tools_stream_final*` (chat streaming; native final text arrives
  as a single chunk) — branches through it.
- `Message` (`conversation/mod.rs`) carries `tool_calls` / `tool_call_id` as skippable
  optionals — conversations persisted before native tool calling still load, and plain
  messages stay clean on the wire.

Operator-facing error conventions for implementors are documented in `provider.rs` module
docs. Wire-fixture tests live beside each provider; the scripted two-turn exchange
(declare → structured calls → tool-role follow-up) is `tests/native_tool_calling.rs`.

### Tool execution model (three sources, one abstraction)

Agents ultimately call **capabilities** that behave like tools. Those capabilities come from **exactly one of three places** (or a deliberate combination), configured per deployment:

| Source | What it is | Examples |
|--------|------------|----------|
| **1. In-repo MCP servers** | Separate processes/crates you ship, registered in config. Transport, framing, and rmcp bootstrap live in [`kowalski-mcp-base`](../kowalski-mcp-base/) (stateless HTTP + stdio). | [`kowalski-mcp-datafusion`](../kowalski-mcp-datafusion/) (DataFusion), [`kowalski-mcp-rookery`](../kowalski-mcp-rookery/) (horde builder over `kowalski-core::rookery`) |
| **2. External MCP (gateway / catalog)** | Third-party or vendor MCP servers the client reaches through a gateway | [Docker MCP Toolkit](https://docs.docker.com/ai/mcp-catalog-and-toolkit/toolkit/) profiles (GitHub, Puppeteer, …), OAuth handled by the gateway |

**Docker MCP gateway (source 2, recommended wiring — PLAN.md §R3):** add **one** stdio server to `[[mcp.servers]]` rather than N individual servers:

```toml
[[mcp.servers]]
name = "docker-mcp"
transport = "stdio"
command = ["docker", "mcp", "gateway", "run"]
```

- Verified: Kowalski's stdio MCP client (`McpStdioClient`) connects to `docker mcp gateway run` and lists tools (`kowalski-cli mcp ping` → `OK`).
- **Two modes.** Default (no flags) = **dynamic**: the gateway exposes management tools (`mcp-find`, `mcp-add`, `mcp-exec`, `code-mode`) so an agent discovers/runs catalog tools on demand — zero config, works out of the box. **Direct** (`--servers <name>` / `--profile <id>`): the named server's tools are exposed by name, but only after that server is installed **and configured (secrets/OAuth)** in Docker Desktop and its container is ready — a one-shot `mcp tools` may report 0 tools until then. Enable servers in Docker Desktop (`docker mcp profile server ls`).
- **`internal` ↔ gateway policy:** `tools/internal/*` (GitHub, web, FS) remain the **dependency-light default / CI path**. When the gateway provides an equivalent (e.g. GitHub, Fetch, Filesystem), it **shadows** the internal tool for that deployment; do not delete the internal fallback.
| **3. Internal tools** | Small, **in-process** helpers under [`src/tools/internal/`](./src/tools/internal/) — fast defaults, strict scope | [`internal/github`](./src/tools/internal/github.rs) (README API + raw fetch), [`internal/web`](./src/tools/internal/web.rs) (plain HTTP path, to grow), [`internal/file_system`](./src/tools/internal/file_system.rs) (bounded FS, planned) |

**Principles (no shortcuts):**

- **How the model invokes tools:** the agent loop uses **native provider tool calling**
  (structured `tool_calls` on the wire) when `[llm] tool_calling` resolves to it, with the
  ReAct JSON-in-text extractor as the fallback for models without tool support — see the
  **LLM providers** section above. Tool schemas come from `ToolDefinition::from_tool` in
  both cases (the prompt appendix and the wire declarations share one owner).
- **Internal tools are not “the platform core”** in a business sense — they are **escape hatches**: dependency-light defaults when no MCP is configured, CI-friendly smoke paths, and deterministic helpers.
- **MCP is the extension plane** for anything that needs OAuth, catalog discovery, headless browsers, vendor APIs, or isolation. The **`McpHub` / `McpClient`** stack ([`src/mcp/`](./src/mcp/)) is the integration point for sources **1** and **2**; do not re-implement those concerns inside `tools/internal/` without a strong reason.
- **Configuration** (future work, explicit in config schema): per **logical capability** (e.g. `fetch_github`, `read_url`, `list_dir`), choose `provider = internal | mcp` and optional `mcp_server` / tool name so operators can **turn internal tools off** or **shadow** them with an MCP tool without changing orchestration code.
- **Naming layout** under `src/tools/`:
  - [`tools/mod.rs`](./src/tools/mod.rs) — `Tool` trait, `ToolInput` / `ToolOutput`, shared types.
  - [`tools/manager.rs`](./src/tools/manager.rs) — registration and dispatch for `Tool` implementations.
  - **`tools/internal/`** — only built-in, in-process families; **one concern per directory** (`github`, `web`, `file_system`, …). No dumping unrelated helpers here.
  - **`src/mcp/`** — MCP **client** protocol, hub, stdio/HTTP transports; **not** a duplicate “tools” tree.

**Call-site rule:** HTTP server (`kowalski`), CLI (`kowalski-cli`), and examples must **not** grow ad-hoc fetch/FS logic; call **`tools::internal::…`** helpers or go through **`ToolManager`** + MCP once wired.

**Operator UI:** behavior visible in the Vue **`ui/`** (chat, horde, federation) must stay coherent with **`/api/*`**; validate smoke paths after changes that affect agents, tools, or federation payloads. See root [`AGENTS.md`](../AGENTS.md) and [`ui/AGENTS.md`](../ui/AGENTS.md).

**[`source_bundle`](./src/source_bundle.rs):** builds `raw/*.md` bundles under the given root (typically `workdir/debug`, so **`debug/raw/`**) from URL / file / text tokens (used by `kowalski-cli` worker ingest and any future server-side ingest). Uses **`tools::internal::github`** and **`tools::internal::web`** (HTML heuristic → Markdown). Not horde-specific.

**[`rookery`](./src/rookery/):** horde builder — `RookeryDraft`, `validate_draft`, `validate_horde_tree`, `write_horde_tree`. Optional DAG scheduling via **`[[edges]]`** in manifest / draft (see [`horde_graph`](./src/horde_graph.rs)). **`write_horde_tree`** emits `[[edges]]` only when the graph differs from an implicit linear chain. Builder system prompt: [`../resources/prompts/rookery/builder.md`](../resources/prompts/rookery/builder.md). Fixtures: `minimal_linear_draft()`, `minimal_dag_draft()`.

**[`horde_graph`](./src/horde_graph.rs):** `HordeEdge`, `resolve_execution_graph()` — validates acyclic graphs, pipeline topological order, and returns scheduling layers. **`execution_order`**, **`next_ready_step`**, **`single_predecessor`** drive CLI/HTTP orchestrators. Empty/missing `edges` → implicit chain along `pipeline` order (linear hordes unchanged). Parallel layers run **sequentially per process** in 1.5.0 MVP.

**Horde manifest `[[edges]]` TOML (optional, 1.5.0+):**

```toml
pipeline = ["ingest", "branch-a", "branch-b", "join", "lint"]

[[edges]]
from = "ingest"
to = "branch-a"

[[edges]]
from = "ingest"
to = "branch-b"

[[edges]]
from = "branch-a"
to = "join"

[[edges]]
from = "branch-b"
to = "join"

[[edges]]
from = "join"
to = "lint"
```

Omit `edges` (or leave empty) for linear hordes — no migration required.

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
Never start a complex task without creating a `task.md` file. Use the template in [`tools/task_template.md`](../tools/task_template.md).

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

### Rule 7: Refactoring is not done until documentation is updated
Any refactor or API change must include **updated docs** in the same delivery: **`AGENTS.md`**, **`README.md`**, **`CHANGELOG.md`**, and **`docs/`** when behavior or architecture changes. **Documentation updates are mandatory task closure.**

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
**1.3.0**: **`rookery`** module (draft validate/write, avatar inference, operator forms), plus existing **`TemplateAgent`**, MCP, federation, and horde apps. See [`../ROADMAP.md`](../ROADMAP.md) and [`ROADMAP.md`](ROADMAP.md).

### Roadmap
See [`ROADMAP.md`](ROADMAP.md) (this crate) and root [`../ROADMAP.md`](../ROADMAP.md).

### Technical Debt
- Legacy sections below describe older multi-crate layout; prefer `src/` tree and root docs.

### Known Issues
- LLM backends must be configured and reachable for live tests.
- Full federation hardening is an ongoing concern (TTL, auth).

---

## Legacy Component Context

Legacy notes for pre-1.1.0 architecture assumptions were moved to:

- [`../docs/purgatory/legacy_v1.1.0.md`](../docs/purgatory/legacy_v1.1.0.md)

Keep this file aligned with the active in-repo architecture (`kowalski-core` as the canonical agent/tool/memory core).

---

## 10. Common AI Tasks

### Code Review Checklist
- [ ] Follows SOLID principles (use [quick reference](../tools/solid_principles_quick_reference.md))
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
