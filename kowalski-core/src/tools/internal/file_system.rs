//! **Internal filesystem** — bounded read/list/copy under configured roots (planned).
//!
//! Security rule: never expose unconstrained host paths to models; roots must come from **config**
//! or operator allowlists. Heavy indexing or full vault sync → **MCP** or a dedicated service.

/// Marker type reserved for future `Tool` registration (`internal_fs_read`, etc.).
#[derive(Debug, Clone, Copy, Default)]
pub struct FileSystemInternalModule;
