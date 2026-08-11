//! Shared debounced filesystem watcher.
//!
//! One `notify` watcher instance covers all watched roots, and one debounce
//! thread coalesces event bursts into a single callback — repeated reloads never
//! create additional watchers, threads, or fds. Used by the horde catalog for
//! hot reload ([`spawn_debounced_watcher`]); the trigger runtime shares one
//! [`DynamicWatcher`] across all its watch triggers (per-trigger debounce lives
//! with the subscriber, which also receives event details).

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
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

/// Filesystem event vocabulary shared with `[[triggers]]` `watch.events`
/// (`kowalski_core::horde_trigger::WATCH_EVENTS`).
#[derive(Debug, Clone)]
pub struct WatchEvent {
    pub path: PathBuf,
    /// `create` / `modify` / `remove`.
    pub kind: &'static str,
}

/// Map a notify event kind onto the trigger vocabulary; `None` for events that
/// cannot change content (pure access) or carry no useful classification.
pub fn watch_event_kind(kind: &notify::EventKind) -> Option<&'static str> {
    use notify::EventKind::*;
    match kind {
        Create(_) => Some("create"),
        Remove(_) => Some("remove"),
        Modify(_) => Some("modify"),
        Access(_) => None,
        // Catch-alls (rename batches, overflow, platform quirks): treat as a
        // modification so subscribers never miss a real change.
        Any | Other => Some("modify"),
    }
}

/// One consumer of a [`DynamicWatcher`]: events under `root` whose kind is in
/// `kinds` are forwarded on `tx`. Debounce/coalescing is the receiver's job.
pub struct WatchSubscription {
    /// Stable identity for logs (e.g. `<horde>#<trigger-index>`).
    pub id: String,
    /// Absolute file or directory this subscription cares about.
    pub root: PathBuf,
    /// Subset of `create` / `modify` / `remove` to forward.
    pub kinds: Vec<String>,
    pub tx: tokio::sync::mpsc::UnboundedSender<WatchEvent>,
}

struct DynamicWatcherInner {
    watcher: RecommendedWatcher,
    /// Paths currently registered with notify (deduped subscription roots).
    watched: HashSet<PathBuf>,
}

/// A single `notify` instance whose watched roots and subscribers can be
/// replaced at runtime — re-arming (e.g. after a horde hot reload) never
/// creates additional watchers, threads, or fds.
pub struct DynamicWatcher {
    inner: Mutex<DynamicWatcherInner>,
    subs: Arc<Mutex<Vec<WatchSubscription>>>,
}

impl DynamicWatcher {
    pub fn new() -> notify::Result<Self> {
        let subs: Arc<Mutex<Vec<WatchSubscription>>> = Arc::new(Mutex::new(Vec::new()));
        let route_subs = subs.clone();
        let watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
                Ok(event) => {
                    let Some(kind) = watch_event_kind(&event.kind) else {
                        return;
                    };
                    let subs = route_subs.lock().expect("watch subscriptions lock poisoned");
                    for path in &event.paths {
                        for sub in subs.iter() {
                            if path.starts_with(&sub.root) && sub.kinds.iter().any(|k| k == kind) {
                                let _ = sub.tx.send(WatchEvent {
                                    path: path.clone(),
                                    kind,
                                });
                            }
                        }
                    }
                }
                Err(e) => log::warn!("fs watcher error: {e}"),
            })?;
        Ok(Self {
            inner: Mutex::new(DynamicWatcherInner {
                watcher,
                watched: HashSet::new(),
            }),
            subs,
        })
    }

    /// Replace the full subscription set and re-point notify at the new roots.
    /// Missing roots are skipped with a warning (they become watchable on the
    /// next re-arm, e.g. after the directory is created and the horde reloaded).
    pub fn set_subscriptions(&self, mut new_subs: Vec<WatchSubscription>) {
        for s in &mut new_subs {
            // Backends report canonical paths (e.g. `/private/var/...` on macOS
            // for a `/var/...` symlink); match and watch the canonical root.
            if let Ok(c) = s.root.canonicalize() {
                s.root = c;
            }
        }
        let desired: HashSet<PathBuf> = new_subs
            .iter()
            .filter(|s| {
                if s.root.exists() {
                    return true;
                }
                log::warn!(
                    "watch trigger {}: path {} does not exist — watch inactive until re-arm",
                    s.id,
                    s.root.display()
                );
                false
            })
            .map(|s| s.root.clone())
            .collect();
        let mut inner = self.inner.lock().expect("dynamic watcher lock poisoned");
        let stale: Vec<PathBuf> = inner.watched.difference(&desired).cloned().collect();
        for p in stale {
            if let Err(e) = inner.watcher.unwatch(&p) {
                log::debug!("fs watcher: unwatch {} failed: {e}", p.display());
            }
            inner.watched.remove(&p);
        }
        let fresh: Vec<PathBuf> = desired.difference(&inner.watched).cloned().collect();
        for p in fresh {
            match inner.watcher.watch(&p, RecursiveMode::Recursive) {
                Ok(()) => {
                    inner.watched.insert(p);
                }
                Err(e) => log::warn!("fs watcher: cannot watch {}: {e}", p.display()),
            }
        }
        *self.subs.lock().expect("watch subscriptions lock poisoned") = new_subs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn dynamic_watcher_routes_by_root_and_rearms() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        // Backends report canonical paths — compare against the same shape.
        let a_canon = a.canonicalize().unwrap();
        let b_canon = b.canonicalize().unwrap();

        let watcher = DynamicWatcher::new().unwrap();
        let (tx_a, mut rx_a) = tokio::sync::mpsc::unbounded_channel();
        watcher.set_subscriptions(vec![WatchSubscription {
            id: "sub-a".into(),
            root: a.clone(),
            kinds: vec!["create".into(), "modify".into()],
            tx: tx_a,
        }]);
        tokio::time::sleep(Duration::from_millis(300)).await;

        std::fs::write(b.join("ignored.md"), "x").unwrap();
        std::fs::write(a.join("seen.md"), "x").unwrap();
        let ev = tokio::time::timeout(Duration::from_secs(10), rx_a.recv())
            .await
            .expect("event within timeout")
            .expect("channel open");
        assert!(ev.path.starts_with(&a_canon), "routed by root: {:?}", ev.path);
        assert!(ev.kind == "create" || ev.kind == "modify", "{}", ev.kind);

        // Re-arm onto root b: same notify instance, new routing. Only events
        // under b reach the new subscription (never the b write from above —
        // subscriptions saw only root a when it happened).
        let (tx_b, mut rx_b) = tokio::sync::mpsc::unbounded_channel();
        watcher.set_subscriptions(vec![WatchSubscription {
            id: "sub-b".into(),
            root: b.clone(),
            kinds: vec!["create".into(), "modify".into()],
            tx: tx_b,
        }]);
        tokio::time::sleep(Duration::from_millis(300)).await;

        std::fs::write(b.join("now-seen.md"), "y").unwrap();
        let ev = tokio::time::timeout(Duration::from_secs(10), rx_b.recv())
            .await
            .expect("event within timeout")
            .expect("channel open");
        assert!(
            ev.path.starts_with(&b_canon),
            "routed to new root: {:?}",
            ev.path
        );
    }

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
