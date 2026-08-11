//! `[[triggers]]` on horde manifests: cron / watch / webhook declarations.
//!
//! This is the **spec + validation layer only** — triggers are parsed from `horde.md`
//! frontmatter, validated, carried on the spec (and its run snapshot), and editable via
//! Rookery. Nothing here schedules or fires runs.

use crate::error::KowalskiError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Filesystem event kinds a `watch` trigger may subscribe to.
pub const WATCH_EVENTS: &[&str] = &["create", "modify", "remove"];

/// Default `watch.events` when the manifest omits them.
pub const DEFAULT_WATCH_EVENTS: &[&str] = &["create", "modify"];

/// Default `watch.debounce_ms` when the manifest omits it.
pub const DEFAULT_WATCH_DEBOUNCE_MS: u64 = 2000;

/// One `[[triggers]]` entry: exactly one of `cron` / `watch` / `webhook`, plus common
/// optional fields. Unknown keys are rejected at parse time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HordeTrigger {
    /// 5-field cron expression (minute hour day-of-month month day-of-week), local timezone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cron: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch: Option<WatchTrigger>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook: Option<WebhookTrigger>,
    /// Disabled triggers still parse and validate; the runtime skips them.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Pre-filled operator-form answers keyed by `run_form` field id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub input: BTreeMap<String, String>,
    /// Run prompt template; placeholders: `{{trigger.path}}`, `{{trigger.payload}}`,
    /// `{{trigger.time}}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

fn default_true() -> bool {
    true
}

/// `watch = { path = "...", events = [...], debounce_ms = ... }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchTrigger {
    /// Watched file or directory; relative paths resolve against the horde root.
    pub path: String,
    /// Subset of [`WATCH_EVENTS`] routed to this trigger.
    #[serde(default = "default_watch_events")]
    pub events: Vec<String>,
    /// Quiet window collapsing a burst of filesystem events into one firing.
    #[serde(default = "default_watch_debounce_ms")]
    pub debounce_ms: u64,
}

fn default_watch_events() -> Vec<String> {
    DEFAULT_WATCH_EVENTS.iter().map(|s| s.to_string()).collect()
}

fn default_watch_debounce_ms() -> u64 {
    DEFAULT_WATCH_DEBOUNCE_MS
}

/// `webhook = { route = "..." }` — the server exposes it under `/api/triggers/<route>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookTrigger {
    /// Route slug (`[a-z0-9][a-z0-9-]*`); must be unique across all hordes.
    pub route: String,
}

impl HordeTrigger {
    /// The declared kind (`cron` / `watch` / `webhook`), or `unset` when none is present
    /// (which [`validate_triggers`] rejects).
    pub fn kind(&self) -> &'static str {
        if self.cron.is_some() {
            "cron"
        } else if self.watch.is_some() {
            "watch"
        } else if self.webhook.is_some() {
            "webhook"
        } else {
            "unset"
        }
    }

    /// Short human summary for listings, e.g. `cron 0 7 * * *` or `webhook my-ingest`.
    pub fn summary(&self) -> String {
        let body = if let Some(c) = &self.cron {
            format!("cron {c}")
        } else if let Some(w) = &self.watch {
            format!("watch {}", w.path)
        } else if let Some(h) = &self.webhook {
            format!("webhook {}", h.route)
        } else {
            "unset".to_string()
        };
        if self.enabled {
            body
        } else {
            format!("{body} (disabled)")
        }
    }
}

/// Lowercase kebab slug: `[a-z0-9][a-z0-9-]*`. Shared shape for horde/step ids and
/// webhook routes.
pub fn is_valid_slug(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Validate a manifest's `[[triggers]]`. `base` (the horde root) anchors relative watch
/// paths for the existence **warning**; pass `None` to skip existence checks (drafts).
/// Returns warnings on success; hard failures aggregate into one `Validation` error.
pub fn validate_triggers(
    triggers: &[HordeTrigger],
    base: Option<&Path>,
) -> Result<Vec<String>, KowalskiError> {
    let mut errs = Vec::new();
    let mut warnings = Vec::new();
    let mut routes: BTreeMap<&str, usize> = BTreeMap::new();

    for (i, t) in triggers.iter().enumerate() {
        let n = i + 1;
        let declared: Vec<&str> = [
            t.cron.as_ref().map(|_| "cron"),
            t.watch.as_ref().map(|_| "watch"),
            t.webhook.as_ref().map(|_| "webhook"),
        ]
        .into_iter()
        .flatten()
        .collect();
        match declared.len() {
            1 => {}
            0 => {
                errs.push(format!(
                    "trigger {n}: must declare exactly one of `cron`, `watch`, or `webhook`"
                ));
                continue;
            }
            _ => {
                errs.push(format!(
                    "trigger {n}: declares {}; exactly one of `cron`/`watch`/`webhook` allowed",
                    declared.join(" + ")
                ));
                continue;
            }
        }

        if let Some(expr) = &t.cron
            && let Err(e) = parse_cron(expr)
        {
            let msg = match &e {
                KowalskiError::Validation(m) => m.clone(),
                other => other.to_string(),
            };
            errs.push(format!("trigger {n}: {msg}"));
        }

        if let Some(w) = &t.watch {
            if w.path.trim().is_empty() {
                errs.push(format!("trigger {n}: watch.path must not be empty"));
            }
            if w.events.is_empty() {
                errs.push(format!(
                    "trigger {n}: watch.events must not be empty (allowed: {})",
                    WATCH_EVENTS.join(", ")
                ));
            }
            for ev in &w.events {
                if !WATCH_EVENTS.contains(&ev.as_str()) {
                    errs.push(format!(
                        "trigger {n}: unknown watch event `{ev}` (allowed: {})",
                        WATCH_EVENTS.join(", ")
                    ));
                }
            }
            if let Some(root) = base
                && !w.path.trim().is_empty()
            {
                let p = Path::new(&w.path);
                let resolved = if p.is_absolute() { p.to_path_buf() } else { root.join(p) };
                if !resolved.exists() {
                    warnings.push(format!(
                        "trigger {n}: watch path `{}` does not exist yet",
                        resolved.display()
                    ));
                }
            }
        }

        if let Some(h) = &t.webhook {
            if !is_valid_slug(&h.route) {
                errs.push(format!(
                    "trigger {n}: webhook route must match [a-z0-9][a-z0-9-]*: `{}`",
                    h.route
                ));
            } else if let Some(first) = routes.insert(h.route.as_str(), n) {
                errs.push(format!(
                    "trigger {n}: duplicate webhook route `{}` (also declared by trigger {first})",
                    h.route
                ));
            }
        }
    }

    if errs.is_empty() {
        Ok(warnings)
    } else {
        Err(KowalskiError::Validation(errs.join("; ")))
    }
}

/// A parsed 5-field cron expression, expanded into per-field value sets. Field order:
/// minute (0–59), hour (0–23), day-of-month (1–31), month (1–12), day-of-week (0–6,
/// Sunday = 0; `7` is accepted and normalized to 0). Numeric values only (no names);
/// items support `*`, `N`, `N-M`, and `/step` on any of those. Timezone is server-local.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronSchedule {
    pub minutes: BTreeSet<u8>,
    pub hours: BTreeSet<u8>,
    pub days_of_month: BTreeSet<u8>,
    pub months: BTreeSet<u8>,
    pub days_of_week: BTreeSet<u8>,
}

pub fn parse_cron(expr: &str) -> Result<CronSchedule, KowalskiError> {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return Err(KowalskiError::Validation(format!(
            "cron `{expr}`: expected 5 fields (minute hour day-of-month month day-of-week), got {}",
            fields.len()
        )));
    }
    let minutes = parse_cron_field(fields[0], "minute", 0, 59)?;
    let hours = parse_cron_field(fields[1], "hour", 0, 23)?;
    let days_of_month = parse_cron_field(fields[2], "day-of-month", 1, 31)?;
    let months = parse_cron_field(fields[3], "month", 1, 12)?;
    let mut days_of_week = parse_cron_field(fields[4], "day-of-week", 0, 7)?;
    if days_of_week.remove(&7) {
        days_of_week.insert(0);
    }
    Ok(CronSchedule {
        minutes,
        hours,
        days_of_month,
        months,
        days_of_week,
    })
}

fn parse_cron_field(
    field: &str,
    name: &str,
    min: u8,
    max: u8,
) -> Result<BTreeSet<u8>, KowalskiError> {
    let err = |detail: String| {
        KowalskiError::Validation(format!("cron {name} field `{field}`: {detail}"))
    };
    let parse_num = |s: &str| -> Result<u8, KowalskiError> {
        let v: u8 = s
            .parse()
            .map_err(|_| err(format!("`{s}` is not a number")))?;
        if v < min || v > max {
            return Err(err(format!("value {v} out of range {min}-{max}")));
        }
        Ok(v)
    };

    let mut out = BTreeSet::new();
    for item in field.split(',') {
        if item.is_empty() {
            return Err(err("empty list item".into()));
        }
        let (range, step) = match item.split_once('/') {
            Some((r, s)) => {
                let step: u16 = s
                    .parse()
                    .map_err(|_| err(format!("step `{s}` is not a number")))?;
                if step == 0 {
                    return Err(err("step must be >= 1".into()));
                }
                (r, step)
            }
            None => (item, 1),
        };
        let (lo, hi) = if range == "*" {
            (min, max)
        } else if let Some((a, b)) = range.split_once('-') {
            let (lo, hi) = (parse_num(a)?, parse_num(b)?);
            if lo > hi {
                return Err(err(format!("range {lo}-{hi} is inverted")));
            }
            (lo, hi)
        } else {
            let n = parse_num(range)?;
            // `N/step` means N through the field max, per standard cron.
            if step > 1 { (n, max) } else { (n, n) }
        };
        let mut v = lo as u16;
        while v <= hi as u16 {
            out.insert(v as u8);
            v += step;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cron_trigger(expr: &str) -> HordeTrigger {
        HordeTrigger {
            cron: Some(expr.into()),
            watch: None,
            webhook: None,
            enabled: true,
            input: BTreeMap::new(),
            prompt: None,
        }
    }

    fn webhook_trigger(route: &str) -> HordeTrigger {
        HordeTrigger {
            cron: None,
            watch: None,
            webhook: Some(WebhookTrigger { route: route.into() }),
            enabled: true,
            input: BTreeMap::new(),
            prompt: None,
        }
    }

    #[test]
    fn parse_cron_accepts_common_expressions() {
        let s = parse_cron("0 7 * * *").unwrap();
        assert_eq!(s.minutes.iter().copied().collect::<Vec<_>>(), vec![0]);
        assert_eq!(s.hours.iter().copied().collect::<Vec<_>>(), vec![7]);
        assert_eq!(s.days_of_month.len(), 31);
        assert_eq!(s.months.len(), 12);
        assert_eq!(s.days_of_week.len(), 7);

        let s = parse_cron("*/15 9-17 1,15 * 1-5").unwrap();
        assert_eq!(
            s.minutes.iter().copied().collect::<Vec<_>>(),
            vec![0, 15, 30, 45]
        );
        assert_eq!(s.hours.len(), 9);
        assert_eq!(s.days_of_month.iter().copied().collect::<Vec<_>>(), vec![1, 15]);
        assert_eq!(s.days_of_week.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn parse_cron_normalizes_sunday_7() {
        let s = parse_cron("0 0 * * 7").unwrap();
        assert_eq!(s.days_of_week.iter().copied().collect::<Vec<_>>(), vec![0]);
    }

    #[test]
    fn parse_cron_rejects_bad_expressions() {
        for (expr, needle) in [
            ("0 7 * *", "expected 5 fields"),
            ("0 7 * * * *", "expected 5 fields"),
            ("75 * * * *", "out of range 0-59"),
            ("* 25 * * *", "out of range 0-23"),
            ("* * 0 * *", "out of range 1-31"),
            ("* * * 13 *", "out of range 1-12"),
            ("* * * * 8", "out of range 0-7"),
            ("*/0 * * * *", "step must be >= 1"),
            ("5-1 * * * *", "range 5-1 is inverted"),
            ("a * * * *", "`a` is not a number"),
            ("1,,2 * * * *", "empty list item"),
        ] {
            let e = parse_cron(expr).unwrap_err().to_string();
            assert!(e.contains(needle), "`{expr}`: expected `{needle}` in `{e}`");
        }
    }

    #[test]
    fn validate_accepts_all_three_kinds() {
        let triggers = vec![
            cron_trigger("0 7 * * *"),
            HordeTrigger {
                cron: None,
                watch: Some(WatchTrigger {
                    path: "inbox".into(),
                    events: vec!["create".into(), "modify".into()],
                    debounce_ms: 2000,
                }),
                webhook: None,
                enabled: true,
                input: BTreeMap::new(),
                prompt: Some("Process {{trigger.path}} at {{trigger.time}}".into()),
            },
            webhook_trigger("my-horde-ingest"),
        ];
        let warnings = validate_triggers(&triggers, None).unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn validate_requires_exactly_one_kind() {
        let mut both = cron_trigger("0 7 * * *");
        both.webhook = Some(WebhookTrigger { route: "r".into() });
        let e = validate_triggers(&[both], None).unwrap_err().to_string();
        assert!(e.contains("cron + webhook"), "{e}");

        let none = HordeTrigger {
            cron: None,
            watch: None,
            webhook: None,
            enabled: true,
            input: BTreeMap::new(),
            prompt: None,
        };
        let e = validate_triggers(&[none], None).unwrap_err().to_string();
        assert!(e.contains("exactly one of `cron`, `watch`, or `webhook`"), "{e}");
    }

    #[test]
    fn validate_rejects_duplicate_routes_and_bad_slugs() {
        let e = validate_triggers(
            &[webhook_trigger("ingest"), webhook_trigger("ingest")],
            None,
        )
        .unwrap_err()
        .to_string();
        assert!(e.contains("duplicate webhook route `ingest`"), "{e}");
        assert!(e.contains("trigger 1"), "{e}");

        let e = validate_triggers(&[webhook_trigger("Bad Route")], None)
            .unwrap_err()
            .to_string();
        assert!(e.contains("[a-z0-9][a-z0-9-]*"), "{e}");
    }

    #[test]
    fn validate_rejects_bad_watch_events_and_warns_on_missing_path() {
        let mut t = cron_trigger("0 7 * * *");
        t.cron = None;
        t.watch = Some(WatchTrigger {
            path: "no-such-dir".into(),
            events: vec!["created".into()],
            debounce_ms: 0,
        });
        let e = validate_triggers(&[t.clone()], None).unwrap_err().to_string();
        assert!(e.contains("unknown watch event `created`"), "{e}");

        t.watch.as_mut().unwrap().events = vec!["create".into()];
        let tmp = std::env::temp_dir();
        let warnings = validate_triggers(&[t], Some(&tmp)).unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("does not exist yet"), "{}", warnings[0]);
    }

    #[test]
    fn toml_round_trip_rejects_unknown_keys_and_fills_defaults() {
        #[derive(serde::Deserialize)]
        struct Doc {
            #[serde(default)]
            triggers: Vec<HordeTrigger>,
        }
        let doc: Doc = toml::from_str(
            r#"
[[triggers]]
cron = "0 7 * * *"

[[triggers]]
watch = { path = "inbox" }
enabled = false

[[triggers]]
webhook = { route = "my-ingest" }
input = { question = "daily digest" }
prompt = "Payload: {{trigger.payload}}"
"#,
        )
        .unwrap();
        assert_eq!(doc.triggers.len(), 3);
        assert!(doc.triggers[0].enabled);
        let w = doc.triggers[1].watch.as_ref().unwrap();
        assert_eq!(w.events, default_watch_events());
        assert_eq!(w.debounce_ms, DEFAULT_WATCH_DEBOUNCE_MS);
        assert!(!doc.triggers[1].enabled);
        assert_eq!(doc.triggers[2].input.get("question").unwrap(), "daily digest");

        let bad = toml::from_str::<Doc>("[[triggers]]\ncron = \"0 7 * * *\"\nfrequency = \"daily\"\n");
        assert!(bad.is_err(), "unknown trigger key must be rejected");
        let bad = toml::from_str::<Doc>("[[triggers]]\nwatch = { path = \"x\", debounce = 5 }\n");
        assert!(bad.is_err(), "unknown watch key must be rejected");
    }
}
