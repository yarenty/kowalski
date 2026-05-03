//! **Internal (built-in) tools** — small, in-process capabilities under explicit directories.
//!
//! Design goals:
//! - Same **logical** surface as MCP-backed tools (fetch URL, read file, …) so callers can swap
//!   **internal** ↔ **MCP** via configuration without rewriting orchestration.
//! - **Not** “the whole platform”: each submodule is intentionally narrow. Prefer **MCP** when you
//!   need OAuth, catalog discovery, headless browsers, or vendor-specific APIs (see
//!   [Docker MCP Toolkit](https://docs.docker.com/ai/mcp-catalog-and-toolkit/toolkit/) for gateway
//!   profiles and external servers).
//! - Future: each family exposes `Tool` implementations + **config toggles** (`tools.internal.*`)
//!   to enable/disable or shadow with an MCP tool name.

pub mod file_system;
pub mod github;
pub mod web;

pub use github::{fetch_url_for_ingest, FetchedUrlBody, GithubFetchKind};
