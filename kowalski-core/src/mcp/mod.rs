//! Model Context Protocol (MCP) client integration in `kowalski-core`.
//!
//! ## Current implementation (WP2)
//!
//! - **`McpClient`**: JSON-RPC 2.0 over **HTTP POST** to a single endpoint (`Config.mcp.servers[].url`).
//!   Sends `initialize`, then `notifications/initialized`, then `tools/list` / `tools/call`.
//! - **`McpHub`**: Connects to multiple servers, merges tool lists, resolves name clashes with
//!   `server_name::tool_name`, and routes `call_tool` to the owning client.
//! - **`McpToolProxy`**: Adapts MCP tools to the core [`crate::tools::Tool`] trait so the existing
//!   `ToolManager` and ReAct loop can execute them.
//! - **System prompt**: [`crate::template::TemplateAgent`] appends `ToolManager::generate_json_schema()`
//!   to the system message when tools are present (see `TemplateAgentConfig::tool_prompt_appendix`).
//!
//! ## Configuration wiring
//!
//! - **`McpServerConfig::headers`**: Applied as `reqwest::Client` default headers (e.g. `Authorization`).
//! - **`McpServerConfig::transport`**: `Http` uses JSON-RPC over POST; `Sse` is logged and still uses POST until a full SSE transport exists.
//! - **Operator CLI**: `cargo run -p kowalski-cli -- mcp ping` loads `[mcp]` from `./config.toml` (or `mcp ping -c /path/to.toml`); other top-level TOML sections in the same file are ignored.
//!
//! ## Follow-ups (not done yet)
//!
//! - **Transport**: Full **SSE** or **Streamable HTTP** session per [MCP transports](https://spec.modelcontextprotocol.io/) — **first priority after core** (`task_plan.md`).
//! - **Lifecycle**: Optionally treat `notifications/initialized` failures as hard errors for strict servers.
//!
//! **Stdio MCP** (`connect_stdio`) and other transports are **deferred** until SSE/streamable HTTP lands — see `rebuild_tasks/wp2_mcp_integration_tasks.md`.
//!
//! **Prompt refresh:** [`crate::template::TemplateAgent::register_tool`] and [`crate::template::TemplateAgent::refresh_tool_prompt_appendix`] update `tool_prompt_appendix` when the tool set changes.
//!
//! ## Tests
//!
//! - `kowalski-core/tests/mcp_client_http_mock.rs` — local Axum mock for `initialize` / `tools/list` / `tools/call`.

pub mod client;
pub mod hub;
pub mod tool;
pub mod types;

pub use client::McpClient;
pub use hub::{McpHub, McpToolBinding};
pub use tool::McpToolProxy;
pub use types::{CallToolResponse, McpToolDescription};
