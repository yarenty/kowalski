//! Model Context Protocol (MCP) client integration in `kowalski-core`.
//!
//! ## Current implementation
//!
//! - **`McpClient`**: JSON-RPC 2.0 over **Streamable HTTP** ([spec](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports)):
//!   every request is an HTTP **POST** to the MCP endpoint with
//!   `Accept: application/json, text/event-stream`; responses may be **`application/json`** or
//!   **`text/event-stream`** (SSE `data:` lines containing JSON-RPC). **`Mcp-Session-Id`** is stored
//!   from responses and sent on later requests. Notifications may receive **202 Accepted**.
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
//! - **`McpServerConfig::transport`**: `Http` and `Sse` both use the Streamable HTTP POST path; `Sse`
//!   indicates the server may reply with SSE bodies (same endpoint).
//! - **Operator CLI**: `cargo run -p kowalski-cli -- mcp ping` loads `[mcp]` from `./config.toml` (or `mcp ping -c /path/to.toml`); other top-level TOML sections in the same file are ignored.
//!
//! ## Follow-ups
//!
//! - **Optional GET listener** for server-initiated messages (open SSE without a preceding POST) — not implemented.
//! - **Stdio MCP**: [`crate::mcp::stdio::McpStdioClient`] — newline JSON-RPC over a subprocess (`McpServerConfig::command`).
//!
//! **Prompt refresh:** [`crate::template::TemplateAgent::register_tool`] and [`crate::template::TemplateAgent::refresh_tool_prompt_appendix`] update `tool_prompt_appendix` when the tool set changes.
//!
//! ## Tests
//!
//! - `kowalski-core/tests/mcp_client_http_mock.rs` — local Axum mock for JSON and SSE responses.

pub mod client;
pub mod hub;
pub mod stdio;
pub mod tool;
pub mod types;

pub use client::McpClient;
pub use hub::{McpConnection, McpHub, McpToolBinding};
pub use stdio::McpStdioClient;
pub use tool::McpToolProxy;
pub use types::{CallToolResponse, McpToolDescription};
