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

**Name**: kowalski  
**Release**: **1.5.0** — thin facade over **`kowalski-core`** with optional **`kowalski-cli`** (`cli` / `full` features).  
**Purpose**: Optional unified dependency entry for apps that want the workspace crates from one package name.  
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
See root **[`../README.md`](../README.md)** for the full tree. At a glance:

```
kowalski/                         # repository root (you are in crate kowalski/)
├── kowalski-core/                # TemplateAgent, tools, memory, MCP, federation
├── kowalski-cli/                 # REPL, operators, extension, agent-app
├── kowalski/                     # This crate: facade `lib` + `kowalski` HTTP binary
├── kowalski-mcp-datafusion/      # Optional standalone MCP (DataFusion)
├── ui/                           # Vue operator UI
├── examples/                     # e.g. knowledge-compiler
├── docs/, tools/, resources/   # SQL migrations live in `kowalski-core/migrations/`
```

There are **no** separate `kowalski-tools`, `kowalski-*-agent`, or `kowalski-federation` crates in this workspace; tools and federation live in **`kowalski-core`**.

### Component-Specific Documentation
**⚠️ CRITICAL**: Read the `AGENTS.md` for the crate you touch:

- [Root AGENTS.md](../AGENTS.md)
- [kowalski-core/AGENTS.md](../kowalski-core/AGENTS.md)
- [kowalski-cli/AGENTS.md](../kowalski-cli/AGENTS.md)
- [kowalski/AGENTS.md](./AGENTS.md) (this crate)
- [kowalski-mcp-datafusion/AGENTS.md](../kowalski-mcp-datafusion/AGENTS.md)
- [ui/AGENTS.md](../ui/AGENTS.md)

**Rule**: Before making changes to any component, **always read its specific AGENTS.md first** to understand:
- Component architecture and responsibilities
- Development workflows and testing approaches  
- API patterns and integration points
- Common issues and troubleshooting steps
- Technology-specific considerations

### Service Architecture
- **`TemplateAgent`** and tools live in **`kowalski-core`**; this crate re-exports **`core`** and optionally **`cli`**.
- **HTTP server** (`kowalski` binary): **`/api/*`** for UI and automation.

#### API auth & CORS (optional, off by default — `src/auth.rs`)

- **Default: auth off, permissive CORS** (single-user local tool — zero setup). Enable with
  the **`--auth`** flag, **`[server] auth = true`** in `config.toml`, or by setting a
  non-empty **`KOWALSKI_API_TOKEN`** env var (an explicit token implies you want auth).
- With auth **enabled**, every `/api/*` request requires **`Authorization: Bearer <token>`**
  (or **`?token=`** for SSE/WebSocket clients that cannot set headers). **`/api/health`**
  always stays open.
- The token is resolved at startup: `KOWALSKI_API_TOKEN` env wins; otherwise the server reads
  (or generates on first start — printed once, persisted **mode 0600**) the file
  **`<config-dir>/db/api_token`** (beside `db/rookery/`).
- Server-spawned workers (`/api/federation/workers/start`, `/api/hordes/{id}/workers/start`)
  inherit both `KOWALSKI_API_TOKEN` (bearer token, when auth is on) and `KOWALSKI_API` (this
  server's real base URL, so workers follow a non-default `--bind`) via `export_worker_env` —
  one helper defines the whole worker contract. Env names + default bind are owned by
  `kowalski_core::config` (`API_TOKEN_ENV`, `API_URL_ENV`, `DEFAULT_API_BIND`) — root
  `AGENTS.md` Rule 8.
- With auth enabled, **CORS** becomes an origin **allowlist** (default: Vite dev UI
  `http://localhost:5173` / `http://127.0.0.1:5173`); configure with repeatable
  `--cors-origin` or `[server] cors_origins = [...]` in `config.toml`. A non-allowlisted
  origin gets no `Access-Control-Allow-Origin` header. One shared token by design (no
  multi-user auth/roles).
#### Horde catalog hot reload (`src/horde.rs` `HordeCatalog`, `src/fswatch.rs`)

- Horde definitions are **no longer frozen at startup**: `HordeManager.catalog`
  (`HordeCatalog::with_roots`) re-resolves a horde from disk when its definition files
  change (stat-only fingerprint of `horde.md` + `agents/*.md` + `prompts/*.md`) and
  re-scans the roots on every listing/`find` — adding, editing, or removing a horde dir
  takes effect **without a restart**. One debounced `notify` watcher over the roots
  (`fswatch::spawn_debounced_watcher`, created once at startup — no watcher growth across
  reloads) additionally pushes rescans and subscribes the orchestrator to new run topics
  (`ensure_topic_subscriptions`).
- **In-flight runs are immune to edits**: the orchestrator advances, resumes, and cancels
  runs from the run's persisted **manifest snapshot** (`HordeSpec::from_snapshot`,
  `spec_for_run`), never the live catalog. A definition change applies from the **next**
  run. (Prompt-file *contents* are read at step-execution time, as before — the snapshot
  pins the run's structure: pipeline, steps, edges, isolation.)
- **Broken edits never drop a horde**: a failed reload keeps the last-good spec and
  surfaces the parse error as `load_error` on `GET /api/hordes` (+ detail); the UI shows a
  ⚠ badge and the error text, and clears it when the file is fixed. A brand-new dir that
  fails to parse is skipped with a warning (same as startup).
- **Workdir lifecycle**: startup keeps the existing policy (`clean_on_startup`, skipped
  while interrupted runs pend resume). A hot-added horde gets its workdir created on
  first reference; a reload **never** re-runs the clean.
- Fixed catalogs (`HordeCatalog::fixed`) serve tests/embedded use — no roots, no reloads.
- **Triggers (declaration layer):** `[[triggers]]` from `horde.md` parse into
  `HordeSpec.triggers` (validated at load — invalid cron / duplicate route / unknown keys /
  unknown `overlap` are a load error like any other manifest problem), ride the manifest
  snapshot, and are exposed read-only on `GET /api/hordes` (+ detail). Webhook routes are a
  **global namespace** (`POST /api/triggers/<route>`): cross-horde collisions are overlaid
  on `load_error` in the listing snapshot (not stored), so they clear as soon as one
  manifest is fixed.

#### Trigger runtime (`src/triggers.rs`)

- **`TriggerManager`** arms every enabled trigger from the catalog at startup and re-arms
  on every catalog hot reload (the same debounced watcher callback that rescans the
  catalog calls `rearm()`). A firing is an ordinary durable run:
  `origin = "trigger"` (`kowalski_core::db::run_store::RUN_ORIGIN_TRIGGER` — auto-resumed
  by the startup resume scan like any non-operator run) with
  `source = "trigger:<cron|watch|webhook>:<horde>"` for provenance, a `trigger_fired`
  event in the run feed, and the trigger's `prompt` template
  (`{{trigger.path}}` / `{{trigger.payload}}` / `{{trigger.time}}`) plus validated
  `input` form answers as the run input. Failures to fire are logged loudly and never
  crash the server.
- **Cron:** the 5-field expressions parsed by `kowalski_core::horde_trigger::parse_cron`
  (hand-rolled, dependency-light) drive a minute scheduler (`spawn_cron_loop`) that checks
  the armed table once per wall-clock minute, local time. `next_cron_match` scans **naive**
  wall-clock time (zone-aware day arithmetic would loop forever on DST fall-back days) and
  is used for the "next fire" log line; `fire_due_cron(now)` is public so tests inject a
  clock instead of sleeping.
- **Watch:** all watch triggers share **one** `notify` instance
  (`fswatch::DynamicWatcher`) whose subscriptions are replaced on re-arm — per-trigger
  debounce/coalescing lives in a small tokio task per trigger, and the deduplicated
  changed paths land in the run input. Relative `watch.path` resolves against the horde
  root; a missing path logs a warning and stays dark until the next re-arm.
- **Webhook:** `POST /api/triggers/{route}` (auth like any `/api/*` route when enabled;
  404 unknown/disabled route, 409 cross-horde collision) resolves the route against the
  live catalog per request — no re-arm needed — and the JSON body becomes
  `{{trigger.payload}}`.
- **Overlap policy** (`overlap = "skip" | "queue" | "parallel"`, default skip): while a
  run from the same trigger is **in flight** — a live orchestrator task owns it (the
  in-memory registry decides; a run parked awaiting input or an interrupted run nobody
  resumed never wedges its trigger) — `skip` drops the firing and records a
  `trigger_skipped` event on the in-flight run; `queue` parks at most one firing (later
  ones coalesce into it) and fires when the run ends — after re-validating the trigger
  still exists and is enabled; `parallel` always starts a run.
- Catalog-watcher callbacks re-arm on every change under the horde roots (run artifacts
  included); `rearm()` compares the serialized trigger view and no-ops when it is
  unchanged, so unrelated churn never resets in-flight debounce windows. If a
  subscription IS replaced mid-burst, the outgoing debounce task fires what it collected
  instead of dropping the events.
- **Operator surface:** `GET /api/hordes/{id}/triggers` returns per-trigger status rows
  (`kind`/`detail`, declared `enabled` vs `effective_enabled` + `overridden`, next cron
  fire, and `last_fired` — the newest run whose `trigger_fired` event names the trigger,
  read from the store so it survives restarts). `POST …/triggers/{index}/enable|disable`
  records an **operator override** persisted at `<config-dir>/db/trigger_overrides.json`
  (keyed `<horde>#<index>`; never edits `horde.md`; an override matching the declaration
  is dropped so future file edits win) and re-arms — overrides are part of the re-arm
  fingerprint, apply to webhook route resolution, and are re-checked before a queued
  firing releases. `POST …/triggers/{index}/fire` is "fire now" for testing: starts the
  run immediately with empty firing details (works on disabled triggers; the overlap
  policy still applies). The `triggers` array on `GET /api/hordes` (+ detail) carries the
  declarations enriched with `index`/`kind`/`detail`/`effective_enabled`/`overridden` so
  the UI's listing poll reflects toggles.

#### Horde run persistence (`src/horde.rs`)

- The orchestrator **writes through** every run/step transition to the persisted run store
  (`kowalski_core::db::run_store::RunStore`, SQLite `runs.sqlite` under `<config-dir>/db/`,
  `KOWALSKI_RUN_DB` override): run created (with a **manifest snapshot** of the loaded
  `HordeSpec`), step delegating/succeeded/failed, per-step `attempt`, loop counts, run
  done/error, and the event feed. The in-memory `RunRegistry` is a cache for the active-run
  hot path; **the DB is the system of record**.
- `RunRecord.status` / `RunStepRecord.status` are the typed store enums
  (`RunStatus` / `StepStatus`). `/api` responses keep the historical wire vocabulary via
  `api_run_status` / `api_step_status` in `src/horde.rs` (`done`→`completed`,
  `error`→`failed`, `succeeded`→`success`) — that mapping is also the readiness vocabulary
  fed to the horde graph. Change it in one place only (root `AGENTS.md` Rule 8).
- Run reads go to the store: `GET /api/hordes/{id}/runs` is paged (`?limit=` ≤ 500,
  `?offset=`, newest first) and returns runs from before a restart; run detail and
  follow-up chat use `HordeManager::persisted_run`. Add **`?status=resumable`** to list
  only interrupted / awaiting-input runs that no live orchestrator task owns.
- **Resume after a restart** (`HordeManager::resume_scan` / `resume_run`): on startup the
  server scans `RunStore::incomplete_runs()`. `awaiting_input` runs are durable by
  construction (waiting ends the executor task) and stay parked; interrupted
  `pending`/`running` runs get a `run_interrupted` feed event and are surfaced as
  `resumable` in run listings. Runs whose **`origin`** is not `operator` (the default for
  UI/API-created runs; `POST /api/hordes/{id}/run` accepts an `origin` body field, e.g.
  `trigger`) are **auto-resumed** by the scan.
- **`POST /api/hordes/{id}/runs/{run_id}/resume`** resumes a run on demand: the step that
  was in flight at the kill is reset as a failed attempt (`attempt` + 1), the next ready
  step is recomputed from the run's **manifest snapshot** graph plus persisted step
  outcomes (loop counts intact — a conditional loop never gains extra iterations), and
  that step is re-delegated. Artifacts of completed steps are never regenerated. A
  `run_resumed` event and an orchestrator feed message mark the resumption; the UI Horde
  tab shows an "Interrupted runs" banner with a Resume button.
- Guard rail: each run spends resume attempts (persisted `resume_count`); after
  **`[horde] resume_max_attempts`** failed resumes (default 2,
  `horde::DEFAULT_RESUME_MAX_ATTEMPTS`) the run goes to `error` with a clear reason.
- **In-process step execution (the default):** `delegate_step` checks the manager's
  `StepHandlerRegistry` (`kowalski_core::horde_step`). Registry hit → the step runs as a
  Tokio task inside the server: `TaskStarted` is published, the handler executes, and the
  outcome comes back as a `TaskFinished` envelope on the run's topic — exactly what a
  federation worker would publish, so `handle_task_finished` stays the single advance
  point. The server registers the deterministic kinds (`verify` / `apply` / `ingest`)
  **and all LLM kinds** (`process`/`step`/`deliver`/`final`, `compile`, `ask`, `lint` —
  `LlmStepHandler`, backed by a dedicated horde `TemplateAgent`, no HTTP self-loopback),
  so standard hordes run with **zero worker processes**; `worker_profiles` skips registry
  kinds.
- **Opt-in process isolation (`isolation = "process"`):** a step whose `agents/*.md`
  frontmatter declares `isolation = "process"` (default `in_process`; validated at load,
  captured in the manifest snapshot) executes in a spawned **one-shot child process**
  instead of the server: `delegate_step` branches to `spawn_isolated_step`, which runs
  `kowalski-cli agent-app exec-step` (resolution: `KOWALSKI_CLI_BIN` env > sibling
  `kowalski-cli` next to the server binary > `cargo run -p kowalski-cli`), writes an
  `IsolatedStepRequest` JSON document to its stdin, republishes `message` lines from its
  stdout as run feed messages, and turns the final `outcome` line into the same
  `TaskFinished` envelope every other path publishes. Lifecycle is owned per step —
  spawn → execute → reap (pid logged) — and both run cancellation and the step timeout
  **kill the child** (no orphans). The child re-loads the server's config file
  (`exec_step_config`) so LLM kinds resolve the same provider/model.
- **Isolation trust model:** the boundary is one OS process, nothing more. It protects
  the server from the step (a crashing/hanging/memory-hungry handler, or a prompt-driven
  tool call, cannot take down or touch server state), and the step gets **no server API
  access at all** — the child talks only stdin/stdout to its parent and (for LLM kinds)
  directly to the configured provider, so there is no loopback to authenticate. File
  scope for LLM tool calls still comes from the existing `sandbox_root` tool policy
  (the operator `project_path`). It does **not** confine filesystem or network access of
  `verify_command`/`apply` beyond ordinary OS permissions — no container, jail, or user
  switching. Use it for third-party hordes you don't fully trust; keep first-party
  hordes in-process (faster, one process).
- **Per-step timeout:** every in-process step runs under **`[horde] step_timeout_secs`**
  (default 600, `horde::DEFAULT_STEP_TIMEOUT_SECS`). A timed-out (or otherwise failed)
  step now **routes over a matching `fail` edge** when the graph has one (loop retries
  with attempt counting); only without a fail route does it fail the run.
- **Cancellation:** **`POST /api/hordes/{id}/runs/{run_id}/cancel`** (optional body
  `{ "reason": … }`) — cancels the run's cooperative `CancellationToken` (the in-flight
  in-process step stops at its next await point), marks that step `cancelled`, remaining
  pending steps `skipped`, and persists the run as `cancelled` with a `run_cancelled`
  event + `RunCancelled` federation message. The UI shows a **Cancel run** button while a
  run is processing. Cancelled runs are terminal (not resumable).
- Startup keeps resumable state intact: a horde with interrupted runs pending resume
  **skips its `clean_on_startup` workdir clean**, so completed steps' artifacts (and
  `@step:` context attachments) survive the restart.

- **Rookery** (`src/rookery.rs`, 1.3.0): horde builder API — see [`../ROADMAP.md`](../ROADMAP.md) (*Planned: Rookery*). Routes (require `Extension` store + running LLM for chat/propose):

| Method | Path | Notes |
|--------|------|--------|
| `GET` | `/api/rookery/sessions` | List all server-owned sessions (newest first) |
| `POST` | `/api/rookery/sessions` | Create session; optional body `{ history?, draft?, summary?, status? }` (legacy restore hint — no longer needed by the UI, see server-owned draft below) |
| `GET` | `/api/rookery/sessions/{id}` | Draft + status |
| `DELETE` | `/api/rookery/sessions/{id}` | Drop session |
| `POST` | `/api/rookery/sessions/{id}/chat` | Body: `{ "message", "stream"? }` — **guided interview turn**: ops phase (delta batch applied to the draft server-side) + reply phase (prose; streams). Response carries `ops_applied` / `ops_error`; SSE adds a `{"type":"ops",…}` event. No tools/memory |
| `POST` | `/api/rookery/sessions/{id}/propose` | Whole-draft fallback: parse `RookeryDraft` TOML/JSON from a one-shot builder reply |
| `POST` | `/api/rookery/sessions/{id}/give-birth` | Body: `{ "output_root"?, "overwrite"? }` → `write_horde_tree` + validate |
| `PATCH` | `/api/rookery/sessions/{id}/penguins/{name}` | Update one penguin in session draft |
| `POST` | `/api/rookery/sessions/{id}/save-horde` | Re-write born horde from draft (overwrite on disk) |
| `POST` | `/api/rookery/sessions/{id}/validate` | Validate draft without birth |
| `GET` | `/api/models` | Ollama model list + server default |

Builder system prompt: [`../resources/prompts/rookery/builder.md`](../resources/prompts/rookery/builder.md). Default birth directory: `examples/` (`KOWALSKI_ROOKERY_OUTPUT`).

**Guided interview (per-turn deltas):** every chat turn runs two channels. The **ops phase**
(`kowalski_core::rookery::run_ops_phase`) asks the model for a small `DeltaBatch` — under
`chat_with_schema` when both `[llm] structured_output` and `[rookery] structured_output` are
on, otherwise fenced-JSON extraction — and applies it to the session draft with prefix-apply
(a rejected op keeps the last-good draft and is reported back). The **reply phase** answers in
prose, informed by an ephemeral system note describing what was applied. The draft `id` is
server-owned: generated per session, re-slugged from `display_name` until birth. Config
(`[rookery]` in `config.toml`, all optional): `max_ops_per_turn` (default 12),
`structured_output` (default true), `allow_replace_draft` (default false — whole-document
emission stays off for small models). The whole-draft propose endpoint and give-birth /
MCP-rookery contracts are unchanged.

**Ollama caveat (empirical):** grammar-constrained decoding over the full multi-op delta
schema can *distort op choice* on Ollama (models avoid `add_step` under the constraint while
emitting perfect batches unconstrained). The default deployment is unaffected —
`[llm] structured_output` is off by default, so the interview uses fenced-JSON extraction,
which is the path that works best with Ollama 7B-class models. Enable the constrained ops
channel only on backends where guided decoding is known-good.

**Server-owned draft (PLAN.md §R1):** the server is the **source of truth** for Rookery sessions. Each session (status, draft, summary, and chat transcript) is persisted as one **YAML** file under the state dir (default `db/rookery/`; override with `KOWALSKI_ROOKERY_STATE`) and reloaded on startup — so sessions survive a server restart **without** the browser re-POSTing the draft. The UI keeps only a thin session-id list and renders draft/status via `GET /api/rookery/sessions/{id}`. The legacy `POST` restore body (`history`/`draft`/…) is still accepted for back-compat but is no longer used by `ui/`.

- **MCP**: client/hub in core; optional **`kowalski-mcp-datafusion`** server for heavy SQL.

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
Any refactor or behavior change must ship with **updated `AGENTS.md` / `README.md` / `CHANGELOG.md` / `docs/`** as appropriate—same PR or stacked immediately after. **Documentation is mandatory closure work.**

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
**1.3.0**: HTTP **`/api/rookery/*`**, horde catalog with **`avatar`** on sub-agents, server-validated **`form_answers`** on horde run; re-exports **`kowalski-core`**; optional **`cli`** feature pulls in **`kowalski-cli`**.

### Roadmap
See [`../ROADMAP.md`](../ROADMAP.md).

### Technical Debt
- Facade crate is easy to overlook; keep `Cargo.toml` feature flags aligned with `kowalski-core` / `kowalski-cli`.

### Known Issues
- Same operational constraints as the core (LLM endpoints, optional Postgres).

---

## Legacy Component Context

Legacy notes for older facade assumptions and pre-1.1.0 module narratives were moved to:

- [`../docs/purgatory/legacy_v1.1.0.md`](../docs/purgatory/legacy_v1.1.0.md)

Keep this file focused on the active facade + HTTP server role of the `kowalski` crate.

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
