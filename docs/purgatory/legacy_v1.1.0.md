# Legacy Context Snapshot (v1.1.0 cleanup)

This file consolidates legacy notes that previously lived inside:

- `kowalski-core/AGENTS.md`
- `kowalski-cli/AGENTS.md`
- `kowalski/AGENTS.md`

Those inline sections were removed to keep active AGENT guides focused on the current workspace shape.

## Historical framing

- Earlier documentation described a larger split into many crates (`kowalski-tools`, `kowalski-*-agent`, `kowalski-federation`, etc.).
- The current workspace is consolidated around:
  - `kowalski-core` (TemplateAgent, tools, memory, MCP, federation types)
  - `kowalski-cli` (operators, extension, agent-app)
  - `kowalski` (facade + HTTP server binary)
  - `kowalski-mcp-datafusion` (optional standalone MCP server)
  - `ui` (Vue operator shell)

## Legacy notes moved from `kowalski-cli/AGENTS.md`

### Prior assumptions

- CLI was described as a primary orchestrator selecting dedicated concrete agents.
- REPL loop details centered on direct concrete agent instantiation.
- “Custom agent path” was framed as scaffolding around older patterns.

### Prior strengths called out

- Immediate runnable UX.
- Simple orchestration loop.
- Demonstrative startup selection menu.

### Prior weaknesses called out

- Tight coupling to concrete agent types.
- Missing scripting mode / richer REPL controls.
- Limited rich output rendering.

### Prior improvement themes

- Config-driven dynamic agent loading.
- Better readline UX.
- Headless scripting mode.
- Rich terminal rendering.

## Legacy notes moved from `kowalski-core/AGENTS.md`

### Prior assumptions

- `tool_chain.rs` and older tool-manager narratives were treated as central architecture.
- Memory and provider coupling were documented against earlier organization assumptions.

### Prior strengths called out

- Strong agent/tool abstractions.
- Multi-tier memory design.
- Async-first Rust architecture.

### Prior weaknesses called out

- Singleton memory provider concerns.
- Tool management fragmentation.
- LLM provider coupling.
- JSON extraction fragility.

### Prior improvement themes

- Dependency-injected memory providers.
- Unified in-core tool manager.
- Better provider abstraction and parsing robustness.

## Legacy notes moved from `kowalski/AGENTS.md`

### Prior assumptions

- Facade crate description referenced old feature families from removed specialized crates.
- Default-feature recommendations referenced superseded module boundaries.

### Prior strengths called out

- Unified API and feature-gated modularity.
- Clear facade pattern value.

### Prior weaknesses called out

- Risk of feature creep.
- Additional abstraction layer overhead.

### Prior improvement themes

- Keep facade feature map minimal and aligned with active crates.
- Keep docs and re-exports synchronized with current workspace structure.

## How to use this file

- Treat this file as a historical reference only.
- For active guidance, use:
  - `AGENTS.md` (root)
  - component `AGENTS.md` files
  - `docs/README.md` and `docs/OVERVIEW_1_1.md`
