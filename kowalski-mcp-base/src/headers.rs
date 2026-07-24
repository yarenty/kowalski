//!
//! Credential forwarding.
//!
//! ## How it flows
//!
//! 1. [`forward_headers_middleware`] runs as an axum layer in front of the
//!    streamable-HTTP MCP service. It copies a configured allow-list of
//!    inbound headers into a [`ForwardedHeaders`] value and inserts it into
//!    the request's `http::Extensions`.
//! 2. The rmcp streamable-HTTP transport propagates request extensions into
//!    each `RequestContext`'s `extensions`.
//! 3. A tool handler reads [`ForwardedHeaders`] back out of
//!    `request_context.extensions` (see [`ForwardedHeaders::from_request_context`])
//!    and forwards the relevant header(s) to the upstream call.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use http::HeaderMap;

/// The set of inbound headers captured for forwarding to the upstream
/// service, for one MCP request.
///
/// Stored in `http::Extensions` by [`forward_headers_middleware`] and read
/// back from `RequestContext::extensions` inside tool handlers.
#[derive(Debug, Clone, Default)]
pub struct ForwardedHeaders {
  /// Lower-cased header name -> value. Lower-casing keeps look-ups
  /// case-insensitive (HTTP header names are case-insensitive).
  values: HashMap<String, String>,
}

impl ForwardedHeaders {
  /// Look up a forwarded header value by (case-insensitive) name.
  pub fn get(&self, name: &str) -> Option<&str> {
    self.values.get(&name.to_ascii_lowercase()).map(String::as_str)
  }

  /// Convenience: the `Authorization` header, if present.
  pub fn authorization(&self) -> Option<&str> {
    self.get("authorization")
  }

  /// True when no headers were forwarded (e.g. an unauthenticated probe).
  pub fn is_empty(&self) -> bool {
    self.values.is_empty()
  }

  /// Build from a raw [`HeaderMap`], keeping only the allow-listed names.
  /// Names in `allow` are matched case-insensitively.
  pub fn from_header_map(headers: &HeaderMap, allow: &[&str]) -> Self {
    let mut values = HashMap::new();
    for name in allow {
      if let Some(v) = headers.get(*name).and_then(|v| v.to_str().ok()) {
        values.insert(name.to_ascii_lowercase(), v.to_string());
      }
    }
    Self { values }
  }

  /// Extract from an rmcp server `RequestContext` inside a tool handler.
  ///
  /// rmcp injects the HTTP request's [`http::request::Parts`] as a single
  /// extension on the `RequestContext`, so the headers captured by
  /// [`forward_headers_middleware`] live in `Parts.extensions` — NOT directly
  /// on `ctx.extensions`. We therefore look both places: directly on the
  /// context (in case a future rmcp merges them) and, failing that, through
  /// the injected `Parts`.
  ///
  /// Returns `ForwardedHeaders::default()` (empty) when the middleware did
  /// not run or no allow-listed headers were present, so callers can treat
  /// "no credentials" uniformly.
  pub fn from_request_context<R>(
    ctx: &rmcp::service::RequestContext<R>,
  ) -> Self
  where
    R: rmcp::service::ServiceRole,
  {
    // 1. Directly on the context extensions.
    if let Some(fwd) = ctx.extensions.get::<ForwardedHeaders>() {
      return fwd.clone();
    }
    // 2. Through the injected HTTP request Parts (the streamable-HTTP path).
    if let Some(parts) = ctx.extensions.get::<http::request::Parts>() {
      if let Some(fwd) = parts.extensions.get::<ForwardedHeaders>() {
        return fwd.clone();
      }
    }
    Self::default()
  }
}

/// Configuration for [`forward_headers_middleware`]: which inbound header
/// names should be captured and forwarded.
#[derive(Debug, Clone)]
pub struct ForwardConfig {
  /// Allow-list of header names to forward (case-insensitive).
  pub allow: Arc<Vec<String>>,
}

impl ForwardConfig {
  /// Build from a list of header names.
  pub fn new<I, S>(names: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    Self { allow: Arc::new(names.into_iter().map(Into::into).collect()) }
  }

  /// The common single-header case: just `Authorization`.
  pub fn authorization_only() -> Self {
    Self::new(["Authorization"])
  }
}

/// Axum middleware that captures the allow-listed inbound headers into a
/// [`ForwardedHeaders`] extension on the request, so downstream MCP tool
/// handlers can forward them to the upstream service.
///
/// Mount it on the router that serves the MCP service:
///
/// ```ignore
/// let app = axum::Router::new()
///     .nest_service("/mcp", mcp_service)
///     .layer(axum::middleware::from_fn_with_state(
///         ForwardConfig::authorization_only(),
///         forward_headers_middleware,
///     ));
/// ```
pub async fn forward_headers_middleware(
  axum::extract::State(config): axum::extract::State<ForwardConfig>,
  mut request: Request,
  next: Next,
) -> Response {
  let allow: Vec<&str> = config.allow.iter().map(String::as_str).collect();
  let forwarded = ForwardedHeaders::from_header_map(request.headers(), &allow);
  request.extensions_mut().insert(forwarded);
  next.run(request).await
}

#[cfg(test)]
mod tests {
  use super::*;

  fn header_map(pairs: &[(&str, &str)]) -> HeaderMap {
    let mut m = HeaderMap::new();
    for (k, v) in pairs {
      m.insert(
        http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
        http::HeaderValue::from_str(v).unwrap(),
      );
    }
    m
  }

  #[test]
  fn captures_only_allowlisted_headers() {
    let headers = header_map(&[
      ("Authorization", "Bearer abc"),
      ("X-Other", "nope"),
      ("X-Doku-User", "alice"),
    ]);
    let fwd = ForwardedHeaders::from_header_map(&headers, &["Authorization", "X-Doku-User"]);
    assert_eq!(fwd.authorization(), Some("Bearer abc"));
    assert_eq!(fwd.get("x-doku-user"), Some("alice"));
    assert_eq!(fwd.get("X-Other"), None, "non-allowlisted header must be dropped");
  }

  #[test]
  fn lookup_is_case_insensitive() {
    let headers = header_map(&[("Authorization", "Bearer abc")]);
    let fwd = ForwardedHeaders::from_header_map(&headers, &["authorization"]);
    assert_eq!(fwd.get("AUTHORIZATION"), Some("Bearer abc"));
    assert_eq!(fwd.get("Authorization"), Some("Bearer abc"));
  }

  #[test]
  fn empty_when_no_allowlisted_headers_present() {
    let headers = header_map(&[("X-Random", "v")]);
    let fwd = ForwardedHeaders::from_header_map(&headers, &["Authorization"]);
    assert!(fwd.is_empty());
    assert_eq!(fwd.authorization(), None);
  }
}
