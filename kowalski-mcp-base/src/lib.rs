//! `kowalski-mcp-base` — shared framework for first-party Kowalski MCP servers.
//!
//! Two server styles share the same conventions (stateless HTTP, output framing, no auth shortcuts):
//!
//! 1. **[`transport::McpHandler`]** — hand-rolled JSON-RPC dispatch; use [`transport::run_stdio`]
//!    or [`transport::serve_http`] (used by `kowalski-mcp-datafusion`, `kowalski-mcp-rookery`).
//! 2. **[`serve`]** — rmcp [`ServerHandler`] + `#[tool_router]` bootstrap at `/mcp` + `/health`
//!    (future servers such as Obsidian).
//!
//! See [`MCP_REQUIREMENTS.md`](./MCP_REQUIREMENTS.md) and [`MANIFEST_SPEC.md`](./MANIFEST_SPEC.md).

pub mod framing;
pub mod headers;
pub mod serve;
pub mod transport;

pub use framing::{FrameKind, frame, structured_framed};
pub use headers::{ForwardConfig, ForwardedHeaders, forward_headers_middleware};
pub use serve::{ServeOptions, serve};
pub use transport::{
    ACCEPT_STREAMABLE, McpHandler, http_router, run_stdio, serve_http, wants_sse,
};

/// Initialise tracing/logging with an `RUST_LOG`-configurable filter, defaulting to `info`.
pub fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
}
