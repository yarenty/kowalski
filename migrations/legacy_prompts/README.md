# Legacy Prompts & Configs

**Docs version 1.1.0**

This directory contains the system prompts, persona configurations, and specialized workflow logic that were extracted from the legacy, specialized agent crates (`kowalski-academic-agent`, `kowalski-code-agent`, `kowalski-data-agent`, `kowalski-web-agent`) before they were deleted in Work Package 1.

These files serve as a reference and will be used as "persona" configurations to seed the generic agent and the Postgres database once the MCP integration (WP2) and Postgres Data Layer (WP3) are completed.


## Horde changes in 1.1.0 (since 1.0.0)

- Legacy prompts continue as migration references while horde-style orchestration moved to markdown app definitions in `examples/knowledge-compiler`.
- Documentation now explicitly positions these prompts against the 1.1.0 horde workflow baseline.
