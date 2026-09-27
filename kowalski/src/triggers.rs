//! Trigger runtime: fires persisted horde runs from cron schedules, filesystem
//! events, and webhook posts.
//!
//! The [`TriggerManager`] arms every enabled `[[triggers]]` declaration in the
//! horde catalog — at startup and again on every catalog hot reload — and turns
//! firings into ordinary durable runs: `origin = "trigger"` (auto-resumed by the
//! startup resume scan) with `source = "trigger:<kind>:<horde>"` for provenance.
//! Firings and skipped firings are recorded as run events for the audit trail;
//! a failure to fire is logged loudly and never takes the server down.
//!
//! Overlap policy per trigger (`overlap` in `[[triggers]]`): while a previous
//! run from the same trigger is still in flight, `skip` (default) drops the
//! firing, `queue` parks at most one firing (later firings coalesce into it),
//! `parallel` always starts a run. "In flight" means a live orchestrator task
//! owns the run — runs parked awaiting operator input, or interrupted runs
//! nobody resumed, never wedge a trigger.

use crate::fswatch::{DynamicWatcher, WatchEvent, WatchSubscription};
use crate::horde::{HordeManager, HordeSpec, RunRecord};
use chrono::{DateTime, Datelike, Local, Timelike};
use kowalski_core::db::run_store::{RUN_ORIGIN_TRIGGER, RunStatus};
use kowalski_core::horde_trigger::{CronSchedule, HordeTrigger, parse_cron};
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// `source` of a trigger-fired run: `trigger:<kind>:<horde>` (provenance for
/// the run row, the UI run feed, and the overlap-policy check).
pub fn trigger_source(kind: &str, horde_id: &str) -> String {
    format!("{}{kind}:{horde_id}", kowalski_core::horde_trigger::TRIGGER_SOURCE_PREFIX)
}

/// Stable identity of one trigger declaration: `<horde>#<index>`. Used for
/// logs, `trigger_fired` events, and the operator override store.
pub fn trigger_key(horde_id: &str, index: usize) -> String {
    format!("{horde_id}#{index}")
}

/// Operator enable/disable overrides, persisted as a small JSON map
/// (`"<horde>#<index>" -> bool`) beside the run store. Toggling a trigger from
/// the UI must survive a restart without editing `horde.md` — the declaration
/// stays authoritative in the file; an override only applies while it differs
/// from the declared `enabled` (toggling back to the declared state removes it).
#[derive(Default)]
struct TriggerOverrides {
    map: BTreeMap<String, bool>,
    /// `None` (tests): overrides live in memory only.
    path: Option<PathBuf>,
}

impl TriggerOverrides {
    fn load(path: PathBuf) -> Self {
        let map = match std::fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str(&raw) {
                Ok(map) => map,
                Err(e) => {
                    log::error!(
                        "trigger overrides {}: unreadable, starting empty: {e}",
                        path.display()
                    );
                    BTreeMap::new()
                }
            },
            Err(_) => BTreeMap::new(), // no file yet
        };
        Self {
            map,
            path: Some(path),
        }
    }

    fn effective(&self, horde_id: &str, index: usize, declared: bool) -> bool {
        self.map
            .get(&trigger_key(horde_id, index))
            .copied()
            .unwrap_or(declared)
    }

    /// Record the operator's choice and persist. Setting a trigger back to its
    /// declared state drops the override so future `horde.md` edits win again.
    fn set(&mut self, horde_id: &str, index: usize, enabled: bool, declared: bool) {
        let key = trigger_key(horde_id, index);
        if enabled == declared {
            self.map.remove(&key);
        } else {
            self.map.insert(key, enabled);
        }
        self.save();
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        let raw = match serde_json::to_string_pretty(&self.map) {
            Ok(raw) => raw,
            Err(e) => {
                log::error!("trigger overrides: serialize failed: {e}");
                return;
            }
        };
        let tmp = path.with_extension("json.tmp");
        let write = std::fs::write(&tmp, raw).and_then(|()| std::fs::rename(&tmp, path));
        if let Err(e) = write {
            log::error!("trigger overrides {}: persist failed: {e}", path.display());
        }
    }
}

/// How often a queued firing re-checks whether the in-flight run finished.
const QUEUE_POLL: Duration = Duration::from_secs(2);

/// Clock the cron scheduler reads. Injectable so tests fire at a chosen minute.
pub trait TriggerClock: Send + Sync {
    fn now_local(&self) -> DateTime<Local>;
}

struct SystemClock;

impl TriggerClock for SystemClock {
    fn now_local(&self) -> DateTime<Local> {
        Local::now()
    }
}

/// Whether `t` (minute precision) matches the schedule. Standard cron day
/// semantics: when both day-of-month and day-of-week are restricted, either
/// matching suffices; an unrestricted (`*`) field defers to the other.
pub fn cron_matches(s: &CronSchedule, t: &DateTime<Local>) -> bool {
    s.minutes.contains(&(t.minute() as u8))
        && s.hours.contains(&(t.hour() as u8))
        && cron_day_matches(s, t)
}

fn cron_day_matches<T: Datelike>(s: &CronSchedule, t: &T) -> bool {
    if !s.months.contains(&(t.month() as u8)) {
        return false;
    }
    let dom = s.days_of_month.contains(&(t.day() as u8));
    let dow = s
        .days_of_week
        .contains(&(t.weekday().num_days_from_sunday() as u8));
    let dom_restricted = s.days_of_month.len() < 31;
    let dow_restricted = s.days_of_week.len() < 7;
    match (dom_restricted, dow_restricted) {
        (true, true) => dom || dow,
        (true, false) => dom,
        (false, true) => dow,
        (false, false) => true,
    }
}

/// First matching minute strictly after `from`, scanning up to `horizon_days`
/// ahead (`None` beyond that — e.g. `0 0 30 2 *` never fires). Used for the
/// "next fire" log line when a cron trigger is armed.
///
/// The scan runs on naive wall-clock time — the clock cron semantics match.
/// Zone-aware arithmetic would be wrong here AND non-terminating: adding
/// `Duration::days(1)` adds 24 absolute hours, so on a DST fall-back day
/// `00:00 + 1 day` lands on `23:00` of the SAME local day and snapping the
/// hour back to 0 loops forever on that date.
pub fn next_cron_match(
    s: &CronSchedule,
    from: &DateTime<Local>,
    horizon_days: u32,
) -> Option<DateTime<Local>> {
    let mut t = from.naive_local().with_second(0)?.with_nanosecond(0)?
        + chrono::Duration::minutes(1);
    let end = from.naive_local() + chrono::Duration::days(i64::from(horizon_days));
    while t <= end {
        if !cron_day_matches(s, &t) {
            // Skip straight to the next calendar day's first minute.
            t = t.date().succ_opt()?.and_hms_opt(0, 0, 0)?;
            continue;
        }
        if !s.hours.contains(&(t.hour() as u8)) {
            t = (t + chrono::Duration::hours(1)).with_minute(0)?;
            continue;
        }
        if !s.minutes.contains(&(t.minute() as u8)) {
            t += chrono::Duration::minutes(1);
            continue;
        }
        return match t.and_local_timezone(Local) {
            // A fold (fall-back) hits the earlier occurrence, like real cron.
            chrono::LocalResult::Single(dt) | chrono::LocalResult::Ambiguous(dt, _) => Some(dt),
            // A DST gap: this wall-clock minute never occurs — keep scanning.
            chrono::LocalResult::None => {
                t += chrono::Duration::minutes(1);
                continue;
            }
        };
    }
    None
}

/// One enabled trigger as armed from the catalog. The runtime works from this
/// snapshot; a catalog reload re-arms everything.
#[derive(Clone)]
struct Armed {
    horde_id: String,
    /// Position in the spec's `[[triggers]]` — stable identity for logs/events.
    index: usize,
    trigger: HordeTrigger,
}

impl Armed {
    fn key(&self) -> String {
        trigger_key(&self.horde_id, self.index)
    }

    fn source(&self) -> String {
        trigger_source(self.trigger.kind(), &self.horde_id)
    }
}

/// Kind-specific firing details substituted into the run prompt.
#[derive(Debug, Clone)]
pub enum Firing {
    Cron,
    Watch { events: Vec<WatchEvent> },
    Webhook { payload: serde_json::Value },
}

/// One trigger's operator status (`GET /api/hordes/{id}/triggers`).
/// `enabled` is always the `horde.md` declaration; `effective_enabled` is what
/// the runtime arms after the operator override.
#[derive(Debug, Serialize)]
pub struct TriggerStatusRow {
    pub index: usize,
    pub kind: String,
    /// Kind + configuration, e.g. `cron 0 7 * * *` / `watch inbox/` / `webhook ingest`.
    pub detail: String,
    pub enabled: bool,
    pub effective_enabled: bool,
    /// True while an operator override differs from the declaration.
    pub overridden: bool,
    pub overlap: String,
    /// Next cron firing (RFC 3339), when armed and within a year.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_fire: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_fired: Option<LastFired>,
}

/// The newest run a trigger fired, for the "last fired" line + run link.
#[derive(Debug, Serialize)]
pub struct LastFired {
    pub run_id: String,
    /// The run's `started_at` timestamp.
    pub time: String,
    pub status: String,
}

/// What one firing did after the overlap policy was applied.
#[derive(Debug)]
pub enum FireOutcome {
    Started(Box<RunRecord>),
    /// Dropped because a run from this trigger is still in flight (`overlap = "skip"`).
    Skipped { active_run_id: String },
    /// Parked behind the in-flight run (`overlap = "queue"`); fires when it ends.
    Queued,
    Failed(String),
}

#[derive(Default)]
struct TriggerState {
    cron: Vec<(CronSchedule, Armed)>,
    /// Firings parked behind an active run (`overlap = "queue"`), at most one
    /// per trigger key; newer firings coalesce by replacing the parked one.
    queued: HashMap<String, Firing>,
    /// Serialized armed set of the last effective re-arm. Catalog watcher
    /// callbacks fire on ANY change under the horde roots (run artifacts,
    /// watched inboxes, …) — re-arming is skipped unless the trigger view
    /// itself changed, so in-flight debounce windows survive unrelated churn.
    armed_fingerprint: String,
}

/// Arms catalog triggers and fires runs. Cheap to clone (shared innards).
#[derive(Clone)]
pub struct TriggerManager {
    manager: HordeManager,
    clock: Arc<dyn TriggerClock>,
    /// `None` when notify initialization failed — watch triggers stay dark
    /// (logged loudly); cron and webhooks still work.
    watcher: Option<Arc<DynamicWatcher>>,
    state: Arc<Mutex<TriggerState>>,
    /// Sync mutex: critical sections are map reads/writes, never held across await.
    overrides: Arc<std::sync::Mutex<TriggerOverrides>>,
}

impl TriggerManager {
    pub fn new(manager: HordeManager) -> Self {
        Self::with_clock(manager, Arc::new(SystemClock))
    }

    pub fn with_clock(manager: HordeManager, clock: Arc<dyn TriggerClock>) -> Self {
        let watcher = match DynamicWatcher::new() {
            Ok(w) => Some(Arc::new(w)),
            Err(e) => {
                log::error!("trigger runtime: fs watcher unavailable — watch triggers inactive: {e}");
                None
            }
        };
        Self {
            manager,
            clock,
            watcher,
            state: Arc::new(Mutex::new(TriggerState::default())),
            overrides: Arc::new(std::sync::Mutex::new(TriggerOverrides::default())),
        }
    }

    /// Persist operator enable/disable overrides at `path` (loading any
    /// existing file). Apply before the first [`Self::rearm`].
    pub fn with_override_store(self, path: PathBuf) -> Self {
        *self.overrides.lock().unwrap() = TriggerOverrides::load(path);
        self
    }

    /// The state the runtime arms: the operator override when one is in force,
    /// else the `enabled` declared in `horde.md`.
    fn effective_enabled(&self, horde_id: &str, index: usize, trigger: &HordeTrigger) -> bool {
        self.overrides
            .lock()
            .unwrap()
            .effective(horde_id, index, trigger.enabled)
    }

    /// (Re-)arm every enabled trigger in the catalog: rebuild the cron table
    /// and replace the watch subscriptions. Called at startup and from the
    /// catalog hot-reload watcher; queued firings survive a re-arm (they
    /// re-validate their trigger against the live catalog before firing).
    /// Webhooks need no arming — routes resolve per-request from the catalog.
    pub async fn rearm(&self) {
        let entries = self.manager.catalog.list();
        // Operator overrides are part of the trigger view: a toggle must
        // re-arm even though the on-disk declarations are unchanged.
        let overrides_view = serde_json::to_string(&self.overrides.lock().unwrap().map)
            .unwrap_or_default();
        let fingerprint = entries
            .iter()
            .map(|e| {
                serde_json::to_string(&(&e.spec.id, &e.spec.root_path, &e.spec.triggers))
                    .unwrap_or_default()
            })
            .chain(std::iter::once(overrides_view))
            .collect::<Vec<_>>()
            .join("\n");
        {
            let mut state = self.state.lock().await;
            if state.armed_fingerprint == fingerprint {
                return;
            }
            state.armed_fingerprint = fingerprint;
        }
        let mut cron: Vec<(CronSchedule, Armed)> = Vec::new();
        let mut subs: Vec<WatchSubscription> = Vec::new();
        let mut watch_count = 0usize;
        for entry in &entries {
            let spec = &entry.spec;
            for (index, trigger) in spec.triggers.iter().enumerate() {
                if !self.effective_enabled(&spec.id, index, trigger) {
                    continue;
                }
                let armed = Armed {
                    horde_id: spec.id.clone(),
                    index,
                    trigger: trigger.clone(),
                };
                if let Some(expr) = &trigger.cron {
                    match parse_cron(expr) {
                        Ok(schedule) => {
                            match next_cron_match(&schedule, &self.clock.now_local(), 366) {
                                Some(next) => log::info!(
                                    "trigger {}: cron `{expr}` armed, next fire {}",
                                    armed.key(),
                                    next.format("%Y-%m-%d %H:%M %Z")
                                ),
                                None => log::warn!(
                                    "trigger {}: cron `{expr}` armed but never fires within a year",
                                    armed.key()
                                ),
                            }
                            cron.push((schedule, armed));
                        }
                        // Catalog specs are validated at load; this is a guard
                        // against skew, not an expected path.
                        Err(e) => log::error!(
                            "trigger {}: invalid cron `{expr}` not armed: {e}",
                            armed.key()
                        ),
                    }
                } else if let Some(watch) = &trigger.watch {
                    let path = Path::new(&watch.path);
                    let root = if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        spec.root_path.join(path)
                    };
                    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                    subs.push(WatchSubscription {
                        id: armed.key(),
                        root: root.clone(),
                        kinds: watch.events.clone(),
                        tx,
                    });
                    log::info!(
                        "trigger {}: watching {} ({})",
                        armed.key(),
                        root.display(),
                        watch.events.join(",")
                    );
                    self.spawn_watch_task(armed, rx);
                    watch_count += 1;
                }
            }
        }
        let cron_count = cron.len();
        self.state.lock().await.cron = cron;
        if let Some(w) = &self.watcher {
            // Replacing the subscriptions drops the previous senders, which
            // ends the previous watch tasks — re-arming leaks nothing.
            w.set_subscriptions(subs);
        } else if !subs.is_empty() {
            log::error!(
                "trigger runtime: {} watch trigger(s) declared but fs watcher is unavailable",
                subs.len()
            );
        }
        log::info!("trigger runtime armed: {cron_count} cron, {watch_count} watch");
    }

    /// Debounce loop for one watch trigger: coalesce an event burst into a
    /// single firing carrying the deduplicated paths. Ends when the
    /// subscription is replaced on re-arm (sender dropped).
    fn spawn_watch_task(
        &self,
        armed: Armed,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<WatchEvent>,
    ) {
        let tm = self.clone();
        let debounce = Duration::from_millis(
            armed
                .trigger
                .watch
                .as_ref()
                .map(|w| w.debounce_ms)
                .unwrap_or_default()
                .max(1),
        );
        tokio::spawn(async move {
            while let Some(first) = rx.recv().await {
                let mut events = vec![first];
                let mut closed = false;
                loop {
                    match tokio::time::timeout(debounce, rx.recv()).await {
                        Ok(Some(ev)) => events.push(ev),
                        // Subscription replaced (re-arm) mid-burst: fire what
                        // was collected instead of dropping the events.
                        Ok(None) => {
                            closed = true;
                            break;
                        }
                        Err(_) => break, // quiet window elapsed
                    }
                }
                let mut seen = std::collections::HashSet::new();
                events.retain(|e| seen.insert(e.path.clone()));
                let outcome = tm.fire(&armed, Firing::Watch { events }).await;
                log_outcome(&armed, &outcome);
                if closed {
                    return;
                }
            }
        });
    }

    /// Fire every armed cron trigger whose schedule matches `now`. Public so
    /// tests can drive the scheduler with an injected time.
    pub async fn fire_due_cron(&self, now: DateTime<Local>) {
        let due: Vec<Armed> = {
            let state = self.state.lock().await;
            state
                .cron
                .iter()
                .filter(|(s, _)| cron_matches(s, &now))
                .map(|(_, a)| a.clone())
                .collect()
        };
        for armed in due {
            let outcome = self.fire(&armed, Firing::Cron).await;
            log_outcome(&armed, &outcome);
        }
    }

    /// Fire a webhook trigger (`spec.triggers[index]`) with the request body as
    /// `{{trigger.payload}}`.
    pub async fn fire_webhook(
        &self,
        spec: &HordeSpec,
        index: usize,
        payload: serde_json::Value,
    ) -> FireOutcome {
        let Some(trigger) = spec.triggers.get(index).cloned() else {
            return FireOutcome::Failed(format!(
                "horde {} has no trigger at index {index}",
                spec.id
            ));
        };
        let armed = Armed {
            horde_id: spec.id.clone(),
            index,
            trigger,
        };
        let outcome = self.fire(&armed, Firing::Webhook { payload }).await;
        log_outcome(&armed, &outcome);
        outcome
    }

    /// [`resolve_webhook_route`] against the live catalog with the operator
    /// overrides applied.
    pub fn resolve_webhook(
        &self,
        route: &str,
    ) -> Result<(Arc<HordeSpec>, usize), WebhookRouteError> {
        let entries = self.manager.catalog.list();
        let overrides = self.overrides.lock().unwrap().map.clone();
        resolve_webhook_route(&entries, route, &overrides)
    }

    /// Operator toggle: override the trigger's enabled state, persist the
    /// override, and re-arm the runtime so the change takes effect immediately.
    pub async fn set_enabled(
        &self,
        horde_id: &str,
        index: usize,
        enabled: bool,
    ) -> Result<(), String> {
        let spec = self
            .manager
            .find(horde_id)
            .ok_or_else(|| format!("unknown horde id: {horde_id}"))?;
        let trigger = spec
            .triggers
            .get(index)
            .ok_or_else(|| format!("horde {horde_id} has no trigger at index {index}"))?;
        self.overrides
            .lock()
            .unwrap()
            .set(horde_id, index, enabled, trigger.enabled);
        self.rearm().await;
        Ok(())
    }

    /// Operator "fire now": start the trigger's run immediately with empty
    /// firing details, still subject to the overlap policy. Deliberately works
    /// on disabled triggers — its purpose is testing a trigger before arming it.
    pub async fn fire_manual(&self, horde_id: &str, index: usize) -> FireOutcome {
        let Some(spec) = self.manager.find(horde_id) else {
            return FireOutcome::Failed(format!("unknown horde id: {horde_id}"));
        };
        let Some(trigger) = spec.triggers.get(index).cloned() else {
            return FireOutcome::Failed(format!(
                "horde {horde_id} has no trigger at index {index}"
            ));
        };
        let firing = match trigger.kind() {
            "watch" => Firing::Watch { events: Vec::new() },
            "webhook" => Firing::Webhook {
                payload: serde_json::Value::Object(Default::default()),
            },
            _ => Firing::Cron,
        };
        let armed = Armed {
            horde_id: spec.id.clone(),
            index,
            trigger,
        };
        let outcome = self.fire(&armed, firing).await;
        log_outcome(&armed, &outcome);
        outcome
    }

    /// Effective enabled state per trigger of `spec`, in declaration order.
    /// Cheap (no store access) — used to enrich catalog listings on every poll.
    pub fn effective_states(&self, spec: &HordeSpec) -> Vec<bool> {
        let overrides = self.overrides.lock().unwrap();
        spec.triggers
            .iter()
            .enumerate()
            .map(|(index, t)| overrides.effective(&spec.id, index, t.enabled))
            .collect()
    }

    /// Per-trigger operator status for one horde (`None`: unknown horde).
    /// `last_fired` comes from the run store — the newest run whose
    /// `trigger_fired` event names this trigger — so it survives restarts.
    pub async fn trigger_status(&self, horde_id: &str) -> Option<Vec<TriggerStatusRow>> {
        let spec = self.manager.find(horde_id)?;
        // Newest-first; one page of history is plenty to locate last firings.
        let runs = self
            .manager
            .persisted_runs(horde_id, 100, 0)
            .await
            .unwrap_or_default();
        let now = self.clock.now_local();
        let rows = spec
            .triggers
            .iter()
            .enumerate()
            .map(|(index, trigger)| {
                let key = trigger_key(&spec.id, index);
                let effective_enabled = self.effective_enabled(&spec.id, index, trigger);
                let last_fired = runs
                    .iter()
                    .find(|r| {
                        r.events.iter().any(|e| {
                            e.get("kind").and_then(|k| k.as_str()) == Some("trigger_fired")
                                && e.get("trigger").and_then(|t| t.as_str())
                                    == Some(key.as_str())
                        })
                    })
                    .map(|r| LastFired {
                        run_id: r.run_id.clone(),
                        time: r.started_at.clone(),
                        status: r.status.as_str().to_string(),
                    });
                let next_fire = if effective_enabled {
                    trigger
                        .cron
                        .as_deref()
                        .and_then(|expr| parse_cron(expr).ok())
                        .and_then(|s| next_cron_match(&s, &now, 366))
                        .map(|t| t.to_rfc3339())
                } else {
                    None
                };
                TriggerStatusRow {
                    index,
                    kind: trigger.kind().to_string(),
                    detail: trigger.detail(),
                    enabled: trigger.enabled,
                    effective_enabled,
                    overridden: effective_enabled != trigger.enabled,
                    overlap: trigger.overlap.clone(),
                    next_fire,
                    last_fired,
                }
            })
            .collect();
        Some(rows)
    }

    /// Apply the trigger's overlap policy, then start the run.
    async fn fire(&self, armed: &Armed, firing: Firing) -> FireOutcome {
        let source = armed.source();
        if armed.trigger.overlap != "parallel"
            && let Some(active_run_id) = self.active_run(&source).await
        {
            if armed.trigger.overlap == "queue" {
                return self.queue_firing(armed, firing, &source).await;
            }
            // skip (default)
            self.manager
                .record_run_event(
                    &active_run_id,
                    json!({
                        "kind": "trigger_skipped",
                        "trigger": armed.key(),
                        "source": source,
                        "time": self.clock.now_local().to_rfc3339(),
                    }),
                )
                .await;
            return FireOutcome::Skipped { active_run_id };
        }
        self.fire_now(armed, firing).await
    }

    /// The run (if any) this trigger must respect under `skip`/`queue`: a run
    /// a live orchestrator task actually owns (in-memory registry — the store
    /// alone can't tell "executing" from "interrupted, awaiting resume", and a
    /// dead incomplete run must never wedge its trigger). Auto-resumed trigger
    /// runs re-enter the registry, so they count; runs parked awaiting operator
    /// input do not.
    async fn active_run(&self, source: &str) -> Option<String> {
        let runs = self.manager.runs.lock().await;
        runs.runs
            .values()
            .find(|r| {
                r.source.as_deref() == Some(source)
                    && matches!(r.status, RunStatus::Pending | RunStatus::Running)
            })
            .map(|r| r.run_id.clone())
    }

    /// Park the firing until the in-flight run ends (at most one parked firing
    /// per trigger; later firings coalesce into it by replacement).
    async fn queue_firing(&self, armed: &Armed, firing: Firing, source: &str) -> FireOutcome {
        let key = armed.key();
        {
            let mut state = self.state.lock().await;
            let already_waiting = state.queued.contains_key(&key);
            state.queued.insert(key.clone(), firing);
            if already_waiting {
                return FireOutcome::Queued;
            }
        }
        let tm = self.clone();
        let armed = armed.clone();
        let source = source.to_string();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(QUEUE_POLL).await;
                if tm.active_run(&source).await.is_none() {
                    break;
                }
            }
            let firing = tm.state.lock().await.queued.remove(&armed.key());
            let Some(firing) = firing else { return };
            // The horde may have been edited while we waited — fire only if
            // the trigger still exists, unchanged in kind, and enabled.
            let still_armed = tm
                .manager
                .find(&armed.horde_id)
                .and_then(|spec| spec.triggers.get(armed.index).cloned())
                .is_some_and(|t| {
                    tm.effective_enabled(&armed.horde_id, armed.index, &t)
                        && t.kind() == armed.trigger.kind()
                });
            if !still_armed {
                log::info!(
                    "trigger {}: queued firing dropped (trigger removed or disabled)",
                    armed.key()
                );
                return;
            }
            let outcome = tm.fire_now(&armed, firing).await;
            log_outcome(&armed, &outcome);
        });
        FireOutcome::Queued
    }

    /// Build the run input from the trigger config + firing details and start
    /// the durable run.
    async fn fire_now(&self, armed: &Armed, firing: Firing) -> FireOutcome {
        let trigger = &armed.trigger;
        let time = self.clock.now_local().to_rfc3339();
        let (paths, payload) = match &firing {
            Firing::Watch { events } => {
                let paths: Vec<String> =
                    events.iter().map(|e| e.path.display().to_string()).collect();
                (paths, String::new())
            }
            Firing::Webhook { payload } => (Vec::new(), payload.to_string()),
            Firing::Cron => (Vec::new(), String::new()),
        };
        let path_str = paths.join("\n");
        let body = match &trigger.prompt {
            Some(template) => template
                .replace("{{trigger.path}}", &path_str)
                .replace("{{trigger.payload}}", &payload)
                .replace("{{trigger.time}}", &time),
            // No template: the event itself is the run input (a watched path is
            // a source the pipeline can ingest; a payload is the raw material;
            // cron falls back to the horde's default question).
            None => match &firing {
                Firing::Watch { .. } => path_str.clone(),
                Firing::Webhook { .. } => payload.clone(),
                Firing::Cron => String::new(),
            },
        };
        // Pre-filled operator-form answers, validated exactly like an operator
        // submission (`POST /api/hordes/{id}/run` keeps the same rules).
        let form_block = if trigger.input.is_empty() {
            None
        } else {
            match self
                .manager
                .find(&armed.horde_id)
                .and_then(|s| s.run_form.clone())
            {
                Some(form) => {
                    match kowalski_core::validate_form_answers(&form, &trigger.input) {
                        Ok(()) => Some(kowalski_core::answers_to_prompt(&form, &trigger.input)),
                        Err(e) => {
                            let msg = format!(
                                "trigger {}: input rejected by the horde's run form, not firing: {e}",
                                armed.key()
                            );
                            log::error!("{msg}");
                            return FireOutcome::Failed(msg);
                        }
                    }
                }
                None => {
                    log::warn!(
                        "trigger {}: has `input` but horde {} declares no run form — ignored",
                        armed.key(),
                        armed.horde_id
                    );
                    None
                }
            }
        };
        let prompt = match &form_block {
            Some(block) if !body.trim().is_empty() => format!("{block}\n\n{body}"),
            Some(block) => block.clone(),
            None => body,
        };
        let question = Some(prompt.trim()).filter(|p| !p.is_empty()).map(String::from);
        let source = armed.source();
        match self
            .manager
            .start_run(
                &armed.horde_id,
                &prompt,
                Some(&source),
                question.as_deref(),
                RUN_ORIGIN_TRIGGER,
            )
            .await
        {
            Ok(run) => {
                let mut event = json!({
                    "kind": "trigger_fired",
                    "trigger": armed.key(),
                    "trigger_kind": trigger.kind(),
                    "time": time,
                });
                if let Firing::Watch { events } = &firing {
                    event["changes"] = json!(
                        events
                            .iter()
                            .map(|e| json!({ "path": e.path.display().to_string(), "kind": e.kind }))
                            .collect::<Vec<_>>()
                    );
                }
                if let Firing::Webhook { payload } = &firing {
                    event["payload"] = payload.clone();
                }
                self.manager.record_run_event(&run.run_id, event).await;
                FireOutcome::Started(Box::new(run))
            }
            Err(e) => FireOutcome::Failed(format!(
                "trigger {}: failed to start run: {e}",
                armed.key()
            )),
        }
    }
}

/// Why a webhook route lookup did not produce a trigger.
#[derive(Debug, PartialEq)]
pub enum WebhookRouteError {
    NotFound,
    /// Route declared by this many hordes — the collision must be fixed first.
    Duplicate(usize),
}

/// Resolve a webhook route slug against the catalog: exactly one enabled
/// webhook trigger must own it. `overrides` maps [`trigger_key`]s to operator
/// enable/disable choices (an overridden-off trigger no longer owns its route).
pub fn resolve_webhook_route(
    entries: &[crate::horde::HordeCatalogEntry],
    route: &str,
    overrides: &BTreeMap<String, bool>,
) -> Result<(Arc<HordeSpec>, usize), WebhookRouteError> {
    let mut matches: Vec<(Arc<HordeSpec>, usize)> = Vec::new();
    for entry in entries {
        for (index, trigger) in entry.spec.triggers.iter().enumerate() {
            let enabled = overrides
                .get(&trigger_key(&entry.spec.id, index))
                .copied()
                .unwrap_or(trigger.enabled);
            if enabled && trigger.webhook.as_ref().is_some_and(|w| w.route == route) {
                matches.push((entry.spec.clone(), index));
            }
        }
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(WebhookRouteError::NotFound),
        n => Err(WebhookRouteError::Duplicate(n)),
    }
}

fn log_outcome(armed: &Armed, outcome: &FireOutcome) {
    match outcome {
        FireOutcome::Started(run) => log::info!(
            "trigger {} ({}) fired run {}",
            armed.key(),
            armed.trigger.summary(),
            run.run_id
        ),
        FireOutcome::Skipped { active_run_id } => log::info!(
            "trigger {}: firing skipped — run {} still in flight (overlap=skip)",
            armed.key(),
            active_run_id
        ),
        FireOutcome::Queued => log::info!(
            "trigger {}: firing queued behind the in-flight run (overlap=queue)",
            armed.key()
        ),
        FireOutcome::Failed(e) => log::error!("trigger {} failed to fire: {e}", armed.key()),
    }
}

/// Minute scheduler: checks the cron table once per wall-clock minute. Firing
/// happens inside [`TriggerManager::fire_due_cron`], so an injected-clock test
/// can drive it without this loop.
pub fn spawn_cron_loop(tm: TriggerManager) {
    tokio::spawn(async move {
        let mut last_checked: Option<(chrono::NaiveDate, u32, u32)> = None;
        loop {
            let now = tm.clock.now_local();
            let minute = (now.date_naive(), now.hour(), now.minute());
            if last_checked != Some(minute) {
                tm.fire_due_cron(now).await;
                last_checked = Some(minute);
            }
            // Sleep past the next minute boundary (small cushion for jitter).
            let to_boundary = 60 - u64::from(now.second()).min(59);
            tokio::time::sleep(Duration::from_millis(to_boundary * 1000 + 200)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horde::{HordeManager, SubAgentSpec};
    use chrono::TimeZone;
    use kowalski_core::db::run_store::{NewRun, PersistedRun, RunStore};
    use kowalski_core::federation::{AgentRegistry, FederationOrchestrator, MpscBroker};
    use kowalski_core::horde_trigger::{WatchTrigger, WebhookTrigger};
    use kowalski_core::resolve_execution_graph;
    use std::collections::BTreeMap;
    use std::path::Path;

    struct FixedClock(DateTime<Local>);

    impl TriggerClock for FixedClock {
        fn now_local(&self) -> DateTime<Local> {
            self.0
        }
    }

    fn local(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(y, mo, d, h, mi, 0).single().unwrap()
    }

    fn trigger(overlap: &str) -> HordeTrigger {
        HordeTrigger {
            cron: None,
            watch: None,
            webhook: None,
            enabled: true,
            overlap: overlap.into(),
            input: BTreeMap::new(),
            prompt: None,
        }
    }

    fn sub(name: &str) -> SubAgentSpec {
        SubAgentSpec {
            name: name.into(),
            kind: "process".into(),
            capability: format!("test.{name}"),
            default_agent_id: format!("agent-{name}"),
            display_name: name.into(),
            description: String::new(),
            prompt_file: None,
            output: None,
            inputs: Vec::new(),
            avatar: None,
            tool_ids: Vec::new(),
            verify_command: None,
            verify_cwd: None,
            apply_mode: None,
            isolation: None,
        }
    }

    fn test_spec(dir: &Path, triggers: Vec<HordeTrigger>) -> HordeSpec {
        let pipeline = vec!["a".to_string()];
        let execution_graph = resolve_execution_graph(&pipeline, None).unwrap();
        HordeSpec {
            id: "test-horde".into(),
            display_name: "Test Horde".into(),
            description: String::new(),
            capability_prefix: "test".into(),
            pipeline,
            manifest_edges: Vec::new(),
            triggers,
            execution_graph,
            default_question: "default question".into(),
            topic: "test.topic".into(),
            artifacts_root: dir.join("artifacts"),
            workdir: dir.join("work"),
            config_on_startup: false,
            delivery_title: String::new(),
            delivery_note: String::new(),
            delivery_root_rel: String::new(),
            delivery_summary_note: String::new(),
            prompt_tip: String::new(),
            category: "other".into(),
            icon: String::new(),
            featured: false,
            root_path: dir.to_path_buf(),
            sub_agents: vec![sub("a")],
            followup_artifact_dir: dir.join("follow"),
            worker_log_dir: dir.join("logs"),
            run_form: None,
        }
    }

    async fn test_tm(spec: HordeSpec, clock: DateTime<Local>) -> TriggerManager {
        let broker = Arc::new(MpscBroker::new());
        let federation = Arc::new(FederationOrchestrator::new(
            Arc::new(AgentRegistry::new()),
            broker.clone(),
        ));
        let store = RunStore::open("sqlite::memory:").await.unwrap();
        let manager = HordeManager::new(vec![spec], broker, federation, store);
        TriggerManager::with_clock(manager, Arc::new(FixedClock(clock)))
    }

    async fn runs_with_source(tm: &TriggerManager, source: &str) -> Vec<PersistedRun> {
        let mut out = Vec::new();
        for run in tm
            .manager
            .persisted_runs("test-horde", 100, 0)
            .await
            .unwrap()
        {
            let full = tm.manager.store.get_run(&run.run_id).await.unwrap().unwrap();
            if full.source.as_deref() == Some(source) {
                out.push(full);
            }
        }
        out
    }

    /// Seed an in-flight run the overlap policy must respect: registered in
    /// the live registry (what `active_run` reads) and present in the store
    /// (where skip events are recorded).
    async fn seed_active_run(tm: &TriggerManager, run_id: &str, source: &str) {
        tm.manager
            .store
            .create_run(&NewRun {
                run_id: run_id.into(),
                horde_id: "test-horde".into(),
                prompt: "occupying".into(),
                source: Some(source.into()),
                question: "q".into(),
                manifest_snapshot: None,
                origin: RUN_ORIGIN_TRIGGER.into(),
            })
            .await
            .unwrap();
        tm.manager
            .store
            .update_run_status(run_id, RunStatus::Running, None)
            .await
            .unwrap();
        let record = RunRecord {
            run_id: run_id.into(),
            horde_id: "test-horde".into(),
            prompt: "occupying".into(),
            source: Some(source.into()),
            question: "q".into(),
            status: RunStatus::Running,
            started_at: "0.000Z".into(),
            finished_at: None,
            current_step_index: 0,
            steps: Vec::new(),
            events: Vec::new(),
            loop_counts: BTreeMap::new(),
            origin: RUN_ORIGIN_TRIGGER.into(),
            resume_count: 0,
            resumable: false,
            title: "q".into(),
            manifest_snapshot: None,
        };
        tm.manager
            .runs
            .lock()
            .await
            .runs
            .insert(run_id.into(), record);
    }

    /// Finish a seeded run in both places, as `handle_task_finished` would.
    async fn finish_seeded_run(tm: &TriggerManager, run_id: &str) {
        tm.manager
            .store
            .update_run_status(run_id, RunStatus::Done, None)
            .await
            .unwrap();
        tm.manager
            .runs
            .lock()
            .await
            .runs
            .get_mut(run_id)
            .unwrap()
            .status = RunStatus::Done;
    }

    #[test]
    fn cron_matches_minute_hour_and_day_semantics() {
        let s = parse_cron("*/15 9-17 * * 1-5").unwrap();
        // 2026-08-10 is a Monday.
        assert!(cron_matches(&s, &local(2026, 8, 10, 9, 0)));
        assert!(cron_matches(&s, &local(2026, 8, 10, 16, 45)));
        assert!(!cron_matches(&s, &local(2026, 8, 10, 9, 5)), "minute off-grid");
        assert!(!cron_matches(&s, &local(2026, 8, 10, 18, 0)), "hour out of range");
        assert!(!cron_matches(&s, &local(2026, 8, 15, 9, 0)), "Saturday excluded");

        // Both day fields restricted: standard cron fires on EITHER match.
        let s = parse_cron("0 0 13 * 5").unwrap();
        assert!(cron_matches(&s, &local(2026, 8, 13, 0, 0)), "13th (a Thursday)");
        assert!(cron_matches(&s, &local(2026, 8, 14, 0, 0)), "a Friday (the 14th)");
        assert!(!cron_matches(&s, &local(2026, 8, 12, 0, 0)), "neither day matches");

        let s = parse_cron("0 12 * 2 *").unwrap();
        assert!(cron_matches(&s, &local(2026, 2, 5, 12, 0)));
        assert!(!cron_matches(&s, &local(2026, 3, 5, 12, 0)), "month restricted");
    }

    #[test]
    fn next_cron_match_scans_forward() {
        let s = parse_cron("30 7 * * *").unwrap();
        let next = next_cron_match(&s, &local(2026, 8, 10, 7, 29), 366).unwrap();
        assert_eq!(next, local(2026, 8, 10, 7, 30));
        // From the scheduled minute itself, the next fire is tomorrow.
        let next = next_cron_match(&s, &local(2026, 8, 10, 7, 30), 366).unwrap();
        assert_eq!(next, local(2026, 8, 11, 7, 30));

        // Weekday schedule crosses the weekend.
        let s = parse_cron("0 9 * * 1-5").unwrap();
        let next = next_cron_match(&s, &local(2026, 8, 14, 10, 0), 366).unwrap();
        assert_eq!(next, local(2026, 8, 17, 9, 0), "Friday 10:00 → Monday 09:00");

        // Feb 30 never exists.
        let s = parse_cron("0 0 30 2 *").unwrap();
        assert_eq!(next_cron_match(&s, &local(2026, 1, 1, 0, 0), 366 * 4), None);
    }

    #[tokio::test]
    async fn cron_trigger_fires_a_persisted_run_at_the_scheduled_minute() {
        let dir = tempfile::tempdir().unwrap();
        let mut t = trigger("skip");
        t.cron = Some("30 7 * * *".into());
        t.prompt = Some("Daily digest at {{trigger.time}}".into());
        let now = local(2026, 8, 10, 7, 30);
        let tm = test_tm(test_spec(dir.path(), vec![t]), now).await;
        tm.rearm().await;

        // Off-schedule minute: nothing fires.
        tm.fire_due_cron(local(2026, 8, 10, 7, 29)).await;
        let source = trigger_source("cron", "test-horde");
        assert!(runs_with_source(&tm, &source).await.is_empty());

        tm.fire_due_cron(now).await;
        let runs = runs_with_source(&tm, &source).await;
        assert_eq!(runs.len(), 1, "exactly one run fired");
        let run = &runs[0];
        assert_eq!(run.origin, RUN_ORIGIN_TRIGGER);
        assert!(
            run.prompt.starts_with("Daily digest at 2026-08-10T07:30:00"),
            "{{trigger.time}} substituted: {}",
            run.prompt
        );
        assert!(
            run.events.iter().any(|e| {
                e.get("kind").and_then(|k| k.as_str()) == Some("trigger_fired")
                    && e.get("trigger").and_then(|t| t.as_str()) == Some("test-horde#0")
            }),
            "trigger_fired event recorded: {:?}",
            run.events
        );
    }

    #[tokio::test]
    async fn disabled_triggers_are_not_armed() {
        let dir = tempfile::tempdir().unwrap();
        let mut t = trigger("skip");
        t.cron = Some("* * * * *".into());
        t.enabled = false;
        let now = local(2026, 8, 10, 7, 30);
        let tm = test_tm(test_spec(dir.path(), vec![t]), now).await;
        tm.rearm().await;
        tm.fire_due_cron(now).await;
        let source = trigger_source("cron", "test-horde");
        assert!(runs_with_source(&tm, &source).await.is_empty());
    }

    #[tokio::test]
    async fn overlap_skip_drops_the_firing_and_records_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut t = trigger("skip");
        t.cron = Some("* * * * *".into());
        let now = local(2026, 8, 10, 7, 30);
        let tm = test_tm(test_spec(dir.path(), vec![t]), now).await;
        tm.rearm().await;
        let source = trigger_source("cron", "test-horde");
        seed_active_run(&tm, "run-active", &source).await;

        tm.fire_due_cron(now).await;

        let runs = runs_with_source(&tm, &source).await;
        assert_eq!(runs.len(), 1, "no new run while one is in flight");
        let active = &runs[0];
        assert!(
            active.events.iter().any(|e| {
                e.get("kind").and_then(|k| k.as_str()) == Some("trigger_skipped")
            }),
            "skip recorded on the in-flight run: {:?}",
            active.events
        );
    }

    #[tokio::test]
    async fn overlap_queue_coalesces_and_fires_after_the_run_ends() {
        let dir = tempfile::tempdir().unwrap();
        let mut t = trigger("queue");
        t.cron = Some("* * * * *".into());
        let now = local(2026, 8, 10, 7, 30);
        let tm = test_tm(test_spec(dir.path(), vec![t]), now).await;
        tm.rearm().await;
        let source = trigger_source("cron", "test-horde");
        seed_active_run(&tm, "run-active", &source).await;

        // Three firings while busy: one queued, the rest coalesce into it.
        tm.fire_due_cron(now).await;
        tm.fire_due_cron(local(2026, 8, 10, 7, 31)).await;
        tm.fire_due_cron(local(2026, 8, 10, 7, 32)).await;
        assert_eq!(runs_with_source(&tm, &source).await.len(), 1);

        finish_seeded_run(&tm, "run-active").await;
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        loop {
            let runs = runs_with_source(&tm, &source).await;
            if runs.len() == 2 {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "queued firing never fired; runs: {}",
                runs.len()
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        // The coalesced queue held exactly one firing.
        tokio::time::sleep(QUEUE_POLL + Duration::from_millis(500)).await;
        assert_eq!(runs_with_source(&tm, &source).await.len(), 2);
    }

    #[tokio::test]
    async fn overlap_parallel_always_fires() {
        let dir = tempfile::tempdir().unwrap();
        let mut t = trigger("parallel");
        t.cron = Some("* * * * *".into());
        let now = local(2026, 8, 10, 7, 30);
        let tm = test_tm(test_spec(dir.path(), vec![t]), now).await;
        tm.rearm().await;
        let source = trigger_source("cron", "test-horde");
        seed_active_run(&tm, "run-active", &source).await;

        tm.fire_due_cron(now).await;
        assert_eq!(runs_with_source(&tm, &source).await.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn watch_trigger_fires_once_per_burst_with_the_path_in_the_input() {
        let dir = tempfile::tempdir().unwrap();
        let inbox = dir.path().join("inbox");
        std::fs::create_dir_all(&inbox).unwrap();
        let mut t = trigger("skip");
        t.watch = Some(WatchTrigger {
            path: "inbox".into(),
            events: vec!["create".into(), "modify".into()],
            debounce_ms: 500,
        });
        let tm = test_tm(
            test_spec(dir.path(), vec![t]),
            local(2026, 8, 10, 7, 30),
        )
        .await;
        tm.rearm().await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        for i in 0..3 {
            std::fs::write(inbox.join(format!("doc{i}.md")), "content").unwrap();
        }

        let source = trigger_source("watch", "test-horde");
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        let runs = loop {
            let runs = runs_with_source(&tm, &source).await;
            if !runs.is_empty() {
                break runs;
            }
            assert!(std::time::Instant::now() < deadline, "watch trigger never fired");
            tokio::time::sleep(Duration::from_millis(200)).await;
        };
        assert_eq!(runs.len(), 1, "burst debounced into one firing");
        let run = &runs[0];
        assert_eq!(run.origin, RUN_ORIGIN_TRIGGER);
        assert!(
            run.prompt.contains("doc") && run.prompt.contains("inbox"),
            "changed path lands in the run input: {}",
            run.prompt
        );
        // The quiet period after the burst starts nothing else.
        tokio::time::sleep(Duration::from_millis(1200)).await;
        assert_eq!(runs_with_source(&tm, &source).await.len(), 1);
    }

    #[tokio::test]
    async fn webhook_fires_with_the_payload_as_input() {
        let dir = tempfile::tempdir().unwrap();
        let mut t = trigger("skip");
        t.webhook = Some(WebhookTrigger {
            route: "my-ingest".into(),
        });
        t.prompt = Some("Payload: {{trigger.payload}}".into());
        let tm = test_tm(
            test_spec(dir.path(), vec![t]),
            local(2026, 8, 10, 7, 30),
        )
        .await;
        let spec = tm.manager.find("test-horde").unwrap();

        let outcome = tm
            .fire_webhook(&spec, 0, json!({"ticket": 42}))
            .await;
        let FireOutcome::Started(run) = outcome else {
            panic!("expected Started, got {outcome:?}");
        };
        assert_eq!(run.source.as_deref(), Some("trigger:webhook:test-horde"));
        let persisted = tm.manager.store.get_run(&run.run_id).await.unwrap().unwrap();
        assert_eq!(persisted.prompt, "Payload: {\"ticket\":42}");
        assert!(
            persisted.events.iter().any(|e| {
                e.get("kind").and_then(|k| k.as_str()) == Some("trigger_fired")
                    && e.get("payload").and_then(|p| p.get("ticket")).is_some()
            }),
            "payload recorded in the audit event: {:?}",
            persisted.events
        );
    }

    #[tokio::test]
    async fn webhook_routes_resolve_to_exactly_one_trigger() {
        let dir = tempfile::tempdir().unwrap();
        let mut hook = trigger("skip");
        hook.webhook = Some(WebhookTrigger { route: "ingest".into() });
        let mut disabled = trigger("skip");
        disabled.webhook = Some(WebhookTrigger { route: "dark".into() });
        disabled.enabled = false;
        let spec = test_spec(dir.path(), vec![hook.clone(), disabled]);
        let mut other = test_spec(dir.path(), vec![hook]);
        other.id = "other-horde".into();

        let catalog = crate::horde::HordeCatalog::fixed(vec![spec.clone()]);
        let entries = catalog.list();
        let none = BTreeMap::new();
        let (found, index) = resolve_webhook_route(&entries, "ingest", &none).unwrap();
        assert_eq!(found.id, "test-horde");
        assert_eq!(index, 0);
        assert_eq!(
            resolve_webhook_route(&entries, "dark", &none).unwrap_err(),
            WebhookRouteError::NotFound,
            "disabled triggers do not serve their route"
        );
        assert_eq!(
            resolve_webhook_route(&entries, "nope", &none).unwrap_err(),
            WebhookRouteError::NotFound
        );

        // Operator overrides flip route ownership without touching horde.md:
        // enabling the declaration-disabled trigger serves its route; disabling
        // the declaration-enabled one releases its route.
        let overrides: BTreeMap<String, bool> = [
            (trigger_key("test-horde", 0), false),
            (trigger_key("test-horde", 1), true),
        ]
        .into();
        let (found, index) = resolve_webhook_route(&entries, "dark", &overrides).unwrap();
        assert_eq!((found.id.as_str(), index), ("test-horde", 1));
        assert_eq!(
            resolve_webhook_route(&entries, "ingest", &overrides).unwrap_err(),
            WebhookRouteError::NotFound
        );

        let collided = crate::horde::HordeCatalog::fixed(vec![spec, other]).list();
        assert_eq!(
            resolve_webhook_route(&collided, "ingest", &none).unwrap_err(),
            WebhookRouteError::Duplicate(2)
        );
    }

    #[tokio::test]
    async fn operator_disable_unarms_and_survives_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("trigger_overrides.json");
        let mut t = trigger("skip");
        t.cron = Some("30 7 * * *".into());
        let now = local(2026, 8, 10, 7, 30);
        let source = trigger_source("cron", "test-horde");

        let tm = test_tm(test_spec(dir.path(), vec![t.clone()]), now)
            .await
            .with_override_store(store_path.clone());
        tm.rearm().await;
        tm.fire_due_cron(now).await;
        assert_eq!(runs_with_source(&tm, &source).await.len(), 1);

        tm.set_enabled("test-horde", 0, false).await.unwrap();
        tm.fire_due_cron(local(2026, 8, 11, 7, 30)).await;
        assert_eq!(
            runs_with_source(&tm, &source).await.len(),
            1,
            "disabled trigger no longer fires"
        );
        let status = tm.trigger_status("test-horde").await.unwrap();
        assert!(status[0].enabled, "declaration untouched");
        assert!(!status[0].effective_enabled);
        assert!(status[0].overridden);
        assert!(status[0].next_fire.is_none(), "no next fire while disabled");

        // "Restart": a fresh manager loading the same override file.
        let tm2 = test_tm(test_spec(dir.path(), vec![t]), now)
            .await
            .with_override_store(store_path);
        tm2.rearm().await;
        tm2.fire_due_cron(now).await;
        assert!(
            runs_with_source(&tm2, &source).await.is_empty(),
            "override survived the restart"
        );

        // Toggling back to the declared state drops the override and re-arms.
        tm2.set_enabled("test-horde", 0, true).await.unwrap();
        let status = tm2.trigger_status("test-horde").await.unwrap();
        assert!(status[0].effective_enabled && !status[0].overridden);
        tm2.fire_due_cron(now).await;
        assert_eq!(runs_with_source(&tm2, &source).await.len(), 1);
    }

    #[tokio::test]
    async fn fire_manual_starts_a_run_and_status_links_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut t = trigger("skip");
        t.cron = Some("30 7 * * *".into());
        let tm = test_tm(test_spec(dir.path(), vec![t]), local(2026, 8, 10, 7, 0)).await;
        tm.rearm().await;

        let outcome = tm.fire_manual("test-horde", 0).await;
        let FireOutcome::Started(run) = outcome else {
            panic!("expected Started, got {outcome:?}");
        };
        assert_eq!(run.source.as_deref(), Some("trigger:cron:test-horde"));
        assert_eq!(run.origin, RUN_ORIGIN_TRIGGER);

        let status = tm.trigger_status("test-horde").await.unwrap();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].kind, "cron");
        assert_eq!(status[0].detail, "cron 30 7 * * *");
        let last = status[0].last_fired.as_ref().expect("last_fired set");
        assert_eq!(last.run_id, run.run_id);
        assert_eq!(
            status[0].next_fire.as_deref(),
            Some(local(2026, 8, 10, 7, 30).to_rfc3339().as_str())
        );

        assert!(matches!(
            tm.fire_manual("test-horde", 7).await,
            FireOutcome::Failed(_)
        ));
        assert!(matches!(
            tm.fire_manual("nope", 0).await,
            FireOutcome::Failed(_)
        ));
    }

    #[tokio::test]
    async fn fire_manual_respects_the_overlap_policy() {
        let dir = tempfile::tempdir().unwrap();
        let mut t = trigger("skip");
        t.cron = Some("30 7 * * *".into());
        let tm = test_tm(test_spec(dir.path(), vec![t]), local(2026, 8, 10, 7, 0)).await;
        tm.rearm().await;
        let source = trigger_source("cron", "test-horde");
        seed_active_run(&tm, "run-busy", &source).await;

        let outcome = tm.fire_manual("test-horde", 0).await;
        assert!(
            matches!(outcome, FireOutcome::Skipped { ref active_run_id } if active_run_id == "run-busy"),
            "manual firing skipped while the trigger's run is in flight: {outcome:?}"
        );
    }
}
