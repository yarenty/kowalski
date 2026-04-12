//! Optional `[agent]` / `[tool]` line prefixes for CLI REPL (`kowalski run`).

use std::cell::Cell;

thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
}

/// When enabled, [`super::Agent::chat_with_tools`] labels LLM lines with `[agent]` and tool rounds with `[tool]`.
pub fn set_repl_trace(enabled: bool) {
    ENABLED.with(|c| c.set(enabled));
}

pub(crate) fn repl_trace_enabled() -> bool {
    ENABLED.with(|c| c.get())
}

/// RAII: enable trace for the current thread until dropped.
pub struct ReplTraceGuard;

impl ReplTraceGuard {
    pub fn enable() -> Self {
        set_repl_trace(true);
        Self
    }
}

impl Drop for ReplTraceGuard {
    fn drop(&mut self) {
        set_repl_trace(false);
    }
}
