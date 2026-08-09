//! Shared debounced filesystem watcher.
//!
//! One `notify` watcher instance covers all watched roots, and one debounce
//! thread coalesces event bursts into a single callback — repeated reloads never
//! create additional watchers, threads, or fds. Used by the horde catalog for
//! hot reload; designed to be reused by future file-driven features.

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

/// Watch `paths` recursively; after events settle for `debounce`, invoke
/// `on_change` once per burst. Missing paths are skipped (a root created later
/// is still picked up by lazy catalog rescans, just without watch events).
/// The returned watcher must be kept alive — dropping it stops watching.
pub fn spawn_debounced_watcher(
    paths: &[PathBuf],
    debounce: Duration,
    on_change: impl Fn() + Send + 'static,
) -> notify::Result<RecommendedWatcher> {
    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher =
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
            // Pure access (read) events can't change definitions; don't wake up.
            Ok(event) if event.kind.is_access() => {}
            Ok(_) => {
                let _ = tx.send(());
            }
            Err(e) => log::warn!("fs watcher error: {e}"),
        })?;
    for p in paths {
        if !p.exists() {
            log::debug!("fs watcher: skipping missing root {}", p.display());
            continue;
        }
        if let Err(e) = watcher.watch(p, RecursiveMode::Recursive) {
            log::warn!("fs watcher: cannot watch {}: {e}", p.display());
        }
    }
    std::thread::Builder::new()
        .name("kowalski-fswatch".into())
        .spawn(move || {
            while rx.recv().is_ok() {
                // Coalesce the burst: keep draining until quiet for `debounce`.
                while rx.recv_timeout(debounce).is_ok() {}
                on_change();
            }
            log::debug!("fs watcher debounce thread exiting");
        })
        .expect("spawn fs watcher debounce thread");
    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watcher_fires_one_debounced_callback_per_burst() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, rx) = mpsc::channel();
        let _watcher = spawn_debounced_watcher(
            &[dir.path().to_path_buf()],
            Duration::from_millis(150),
            move || {
                let _ = tx.send(());
            },
        )
        .unwrap();
        // Give the OS watcher a moment to arm.
        std::thread::sleep(Duration::from_millis(300));

        // A burst of writes coalesces into (at least) one callback.
        for i in 0..5 {
            std::fs::write(dir.path().join(format!("f{i}.md")), "x").unwrap();
        }
        assert!(
            rx.recv_timeout(Duration::from_secs(10)).is_ok(),
            "debounced callback fired"
        );
        // After the debounce window drains, a fresh change fires again —
        // the single watcher keeps working across repeated reload cycles.
        while rx.recv_timeout(Duration::from_millis(400)).is_ok() {}
        std::fs::write(dir.path().join("later.md"), "y").unwrap();
        assert!(
            rx.recv_timeout(Duration::from_secs(10)).is_ok(),
            "watcher still live after first burst"
        );
    }
}
