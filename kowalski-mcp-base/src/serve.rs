//!
//! Streamable-HTTP server bootstrap.
//!
//! [`serve`] wires together:
//!   * the rmcp `StreamableHttpService` mounted at `/mcp`,
//!   * the [`super::headers::forward_headers_middleware`] that captures
//!     per-user credential headers for forwarding,
//!   * a `/health` endpoint,
//!   * graceful shutdown on Ctrl-C / cancellation.
//!

use std::net::SocketAddr;

use rmcp::handler::server::ServerHandler;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::StreamableHttpService;
use rmcp::transport::StreamableHttpServerConfig;
use tokio_util::sync::CancellationToken;

use crate::headers::{forward_headers_middleware, ForwardConfig};

const STATEFUL_MODE: bool = false;

/// Options for [`serve`].
pub struct ServeOptions {
  /// Address to bind, e.g. `0.0.0.0:8000`.
  pub bind: SocketAddr,
  /// Which inbound headers to forward to the upstream service per request.
  pub forward: ForwardConfig,
}

impl ServeOptions {
  /// Build options from a bind address and the common "forward only
  /// Authorization" header policy.
  pub fn new(bind: SocketAddr) -> Self {
    Self { bind, forward: ForwardConfig::authorization_only() }
  }

  /// Override the forwarded-header allow-list (e.g. for MCPs that use
  /// `X-*` credential headers instead of / in addition to Authorization).
  pub fn with_forward(mut self, forward: ForwardConfig) -> Self {
    self.forward = forward;
    self
  }
}

/// Run a first-party MCP server over streamable-HTTP until shutdown.
///
/// `make_handler` is called once per session to build a fresh handler
/// instance (rmcp's session model). The handler must implement
/// [`ServerHandler`] (typically via `#[tool_router]` + `#[tool]`).
///
/// Per-user credential headers named in `options.forward` are captured by
/// middleware and made available to tool handlers via
/// [`crate::headers::ForwardedHeaders::from_request_context`].
pub async fn serve<H, F>(options: ServeOptions, make_handler: F) -> anyhow::Result<()>
where
  H: ServerHandler + Send + 'static,
  F: Fn() -> H + Send + Sync + 'static,
{
  let ct = CancellationToken::new();

  let service = StreamableHttpService::new(
    move || Ok(make_handler()),
    LocalSessionManager::default().into(),
    StreamableHttpServerConfig::default()
      // Stateless: no `initialize`/`Mcp-Session-Id` handshake. Each tool call
      // is a self-contained POST so per-request credential forwarding works
      // and tool discovery (unauthenticated `tools/list`) still succeeds.
      .with_stateful_mode(STATEFUL_MODE)
      // Return plain JSON (no SSE framing) for simple request/response tools,
      // which is what the gateway's sessionless path expects.
      .with_json_response(!STATEFUL_MODE)
      .with_cancellation_token(ct.child_token())
      // DNS-rebinding protection via Host header validation is unnecessary
      .disable_allowed_hosts(),
  );

  let app = axum::Router::new()
    .route("/health", axum::routing::get(health))
    .nest_service("/mcp", service)
    .layer(axum::middleware::from_fn_with_state(
      options.forward.clone(),
      forward_headers_middleware,
    ));

  let listener = tokio::net::TcpListener::bind(options.bind).await?;
  tracing::info!(bind = %options.bind, "MCP streamable-HTTP server listening on /mcp");

  let shutdown = {
    let ct = ct.clone();
    async move {
      let _ = tokio::signal::ctrl_c().await;
      tracing::info!("shutdown signal received");
      ct.cancel();
    }
  };

  axum::serve(listener, app).with_graceful_shutdown(shutdown).await?;
  Ok(())
}

/// Minimal liveness endpoint.
async fn health() -> &'static str {
  "OK"
}
