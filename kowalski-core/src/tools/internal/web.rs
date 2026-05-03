//! **Internal web** — plain HTTP fetch, size limits, and (later) optional HTML normalization.
//!
//! Keep this **small**: anything requiring JavaScript rendering, captchas, or authenticated
//! scraping belongs behind an **MCP server** (self-hosted or via the
//! [Docker MCP Toolkit](https://docs.docker.com/ai/mcp-catalog-and-toolkit/toolkit/) gateway).

/// Marker type reserved for future `Tool` registration (`internal_web_fetch`, etc.).
#[derive(Debug, Clone, Copy, Default)]
pub struct WebInternalModule;
