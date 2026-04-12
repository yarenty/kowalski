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
//! ## Follow-ups (not done yet)
//!
//! - **Transport**: Full **SSE** (or Streamable HTTP) session per [MCP transports](https://spec.modelcontextprotocol.io/);
//!   wire `McpServerConfig::transport` and `headers` into `McpClient`.
//! - **Lifecycle**: Optionally treat `notifications/initialized` failures as hard errors for strict servers.
//! - **Prompt refresh**: Recompute `tool_prompt_appendix` when tools are registered after agent construction
//!   (e.g. `TemplateAgent::register_tool`).
//! - **CLI**: `mcp ping` / health command (WP6-style operator UX).
//! - **Stdio MCP**: `McpClient::connect_stdio` for local processes (see WP2 task file “Later” section).
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
