//! **Internal (built-in) tools** — small, in-process capabilities under explicit directories.
//!
//! **Raw source bundling** (URLs + files → one markdown artifact under `raw/`) lives at
//! crate root [`source_bundle`](../../source_bundle.rs), not in `kowalski-cli`, so surface binaries
//! stay thin.
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

pub use file_system::{
    append_file, copy_file, file_len, is_dir, is_file, list_dir_entries, list_dir_names, mkdir_all,
    path_exists, read_file_bounded, read_file_prefix, remove_file, rename, try_canonicalize,
    write_file, DEFAULT_MAX_READ_BYTES, FileSystemInternalModule,
};
pub use github::{fetch_url_for_ingest, FetchedUrlBody, GithubFetchKind, resolve_github_fetch};
pub use web::{fetch_url_as_markdown, html_body_to_markdown, looks_like_html};
