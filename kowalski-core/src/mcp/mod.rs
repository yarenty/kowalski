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
//!
//! ## Follow-ups (not done yet)
//!
//! - **Transport**: Full **SSE** (or Streamable HTTP) session per [MCP transports](https://spec.modelcontextprotocol.io/).
//! - **Lifecycle**: Optionally treat `notifications/initialized` failures as hard errors for strict servers.
//! - **CLI**: `mcp ping` / health command (WP6-style operator UX).
//! - **Stdio MCP**: `McpClient::connect_stdio` for local processes (see WP2 task file “Later” section).
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
