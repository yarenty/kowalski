//! Chat that knows kowalski: before every Chat reply the conversation's system message gets a
//! short live snapshot of this install and, when the message is about kowalski, the sections of
//! the built-in user guide (`docs/guide/`) that match it. The model answers from those facts,
//! not from memory, and links to the screens in the app. Read-only: it explains and links.
//!
//! The snapshot never holds a secret: keys are reported as set or not set.

use crate::http_api::ApiState;
use std::fmt::Write as _;

/// How many guide sections go with a message about kowalski.
const GUIDE_SECTIONS: usize = 3;

/// One `##` section of a guide page (or the page's lead before its first `##`).
#[derive(Debug, Clone, PartialEq)]
pub struct GuideSection {
    /// "Page title › Section title".
    pub title: String,
    pub text: String,
}

/// Every section of the built-in guide, in page order.
pub fn guide_sections() -> Vec<GuideSection> {
    crate::embedded::GUIDE_FILES
        .iter()
        .filter_map(|(_, bytes)| std::str::from_utf8(bytes).ok())
        .flat_map(sections_of)
        .collect()
}

fn sections_of(page: &str) -> Vec<GuideSection> {
    let page_title = page.lines().find_map(|l| l.strip_prefix("# ")).unwrap_or("Guide").trim().to_string();
    let mut out = Vec::new();
    let mut title = page_title.clone();
    let mut text = String::new();
    for line in page.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            if !text.trim().is_empty() {
                out.push(GuideSection { title: title.clone(), text: text.trim().to_string() });
            }
            title = format!("{page_title} › {}", h.trim());
            text.clear();
        } else if !line.starts_with("# ") {
            text.push_str(line);
            text.push('\n');
        }
    }
    if !text.trim().is_empty() {
        out.push(GuideSection { title, text: text.trim().to_string() });
    }
    out
}

/// Lower-case words worth matching on (common English words and very short words dropped).
fn words(text: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "and", "for", "you", "your", "with", "that", "this", "what", "how", "can", "does", "from", "are",
        "was", "why", "into", "its", "it's", "have", "has", "not", "but", "use", "get", "one", "all", "any",
        "when", "where", "who", "which", "will", "would", "should", "could", "about", "then", "them", "they",
        "there", "here", "out", "our", "now", "just", "like", "want", "need", "make", "please", "kowalski",
    ];
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .map(|w| w.trim_matches('-'))
        .filter(|w| w.len() >= 3 && !STOP.contains(w))
        .map(stem)
        .collect()
}

/// A rough stem, enough to meet "failed" with "fail" and "schedules" with "schedule".
fn stem(word: &str) -> String {
    let w = word.trim_end_matches('s');
    for suffix in ["ing", "ed"] {
        if let Some(root) = w.strip_suffix(suffix)
            && root.len() >= 3
        {
            return root.to_string();
        }
    }
    w.to_string()
}

/// The guide sections that best match `question`, best first; empty when nothing matches (the
/// message is not about kowalski, or the guide has nothing on it).
pub fn relevant_sections(question: &str, sections: &[GuideSection], max: usize) -> Vec<GuideSection> {
    let asked: std::collections::HashSet<String> = words(question).into_iter().collect();
    if asked.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(usize, &GuideSection)> = sections
        .iter()
        .map(|s| {
            let title: std::collections::HashSet<String> = words(&s.title).into_iter().collect();
            let body: std::collections::HashSet<String> = words(&s.text).into_iter().collect();
            let score = asked.iter().map(|w| 3 * usize::from(title.contains(w)) + usize::from(body.contains(w))).sum();
            (score, s)
        })
        .filter(|(score, _)| *score >= 2)
        .collect();
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    scored.into_iter().take(max).map(|(_, s)| s.clone()).collect()
}

/// A short, secret-free picture of this install: version, model, folders, search, tableski,
/// hordes with their schedules, and the latest failed runs.
pub async fn snapshot(state: &ApiState, tool_names: &[String]) -> String {
    let cfg = &state.full_config;
    let mut s = String::new();
    let _ = writeln!(s, "- kowalski {} · config {}", env!("CARGO_PKG_VERSION"), state.config_path.display());
    let provider = cfg.llm.provider.as_str();
    let key = |set: bool| if set { "set" } else { "not set" };
    let _ = writeln!(
        s,
        "- model: {} via {}{}",
        state.model,
        provider,
        if provider == "openai" { format!(" (API key {})", key(cfg.llm.openai_api_key.as_deref().is_some_and(|k| !k.is_empty()))) } else { String::new() }
    );
    let _ = writeln!(s, "- files folder: {}", crate::setup::files_dir(cfg).unwrap_or_else(|| "not set".into()));
    let _ = writeln!(
        s,
        "- notes vault: {}",
        kowalski_core::config::vault_dir(cfg).map_or("not set (set it in Setup)".to_string(), |p| {
            let notes = p.join(kowalski_core::vault::VAULT_FOLDER);
            format!("{}; the Knowledge Compiler and the Morning brief save their notes in {}", p.display(), notes.display())
        })
    );
    let search = kowalski_core::tools::internal::SearchBackend::from_config(cfg).map_or("off", |b| b.name());
    let _ = writeln!(s, "- web search: {search}");
    let tableski = cfg.mcp.servers.iter().find(|m| m.name == crate::setup::TABLESKI_SERVER_NAME);
    let tableski_tools = tool_names.iter().any(|t| t == "list_tables" || t == "query_sql");
    let _ = writeln!(
        s,
        "- tableski: {}",
        match tableski {
            None => "not connected".to_string(),
            Some(t) => format!(
                "{}{}, tools {}",
                t.url,
                if t.oauth.is_some() { " (signed in)" } else { "" },
                if tableski_tools { "available" } else { "not available (not reachable yet)" }
            ),
        }
    );
    let _ = writeln!(s, "- tools in Chat: {}", if tool_names.is_empty() { "none".to_string() } else { tool_names.join(", ") });

    let _ = writeln!(s, "- hordes (the user runs them from the Hordes screen; Chat cannot run them):");
    for entry in state.horde_manager.catalog.list() {
        let spec = &entry.spec;
        let triggers = state.trigger_manager.trigger_status(&spec.id).await.unwrap_or_default();
        let schedules: Vec<String> = triggers
            .iter()
            .map(|t| format!("{} is {}", t.detail, if t.effective_enabled { "on" } else { "off" }))
            .collect();
        let _ = writeln!(
            s,
            "  - {} (id {}){}{}",
            spec.display_name,
            spec.id,
            if schedules.is_empty() { String::new() } else { format!(" · {}", schedules.join(", ")) },
            entry.load_error.as_ref().map(|e| format!(" · LOAD ERROR: {e}")).unwrap_or_default()
        );
    }

    let failed = state
        .horde_manager
        .store
        .list_runs_filtered(None, &[kowalski_core::db::run_store::RunStatus::Error], 3, 0)
        .await
        .unwrap_or_default();
    if failed.is_empty() {
        let _ = writeln!(s, "- recent failed runs: none");
    } else {
        let _ = writeln!(s, "- recent failed runs:");
        for run in failed {
            let reason = state
                .horde_manager
                .store
                .get_run(&run.run_id)
                .await
                .ok()
                .flatten()
                .and_then(|r| {
                    r.events.iter().rev().find(|e| e["kind"] == "run_failed").and_then(|e| e["reason"].as_str().map(str::to_string))
                })
                .unwrap_or_else(|| "no reason recorded".into());
            let title = kowalski_core::run_title(&run.prompt, &run.question, run.source.as_deref());
            let reason: String = reason.chars().take(240).collect();
            let _ = writeln!(s, "  - {} \"{}\" ({}): {}", run.horde_id, title, run.run_id, reason);
        }
    }
    s
}

/// The context for one Chat reply: who the assistant is, the live snapshot, and the guide
/// sections that match `message`.
pub fn chat_context(snapshot: &str, message: &str, sections: &[GuideSection]) -> String {
    let mut c = String::from(
        "You are Kowalski's own assistant, running inside the user's kowalski. When a question is about \
kowalski itself (installing, Setup, models, hordes, runs and why they failed, schedules, tableski, the \
notes vault, building a horde), answer only from the facts and guide below, and when the facts give the value \
asked about (a path, the model, whether a schedule is on), state that value; if they don't cover it, say \
so plainly and point to the guide. Never invent settings, menus or commands. Link to the app's screens \
with these Markdown links: [Hordes](?tab=federation-run), [Runs](?tab=runs), [Chat](?tab=chat), \
[Build](?tab=rookery), [Setup](?tab=setup), and a horde's page as [Its name](?horde=<id>). You can \
explain and link, but you cannot change settings or start runs yourself. Other questions: answer them \
normally.\n\n## This kowalski right now\n",
    );
    c.push_str(snapshot);
    let picked = relevant_sections(message, sections, GUIDE_SECTIONS);
    if !picked.is_empty() {
        c.push_str("\n## From the kowalski guide\n");
        for section in picked {
            let _ = write!(c, "\n### {}\n{}\n", section.title, section.text);
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<GuideSection> {
        let page = "# Setup\n\nIntro.\n\n## 1. Model\n\nOllama on your machine; ollama pull gemma4:e2b.\n\n## 3. tableski (optional)\n\nConnect tableski signs you in.\n";
        let mut s = sections_of(page);
        s.extend(sections_of("# Notes vault (Obsidian)\n\nSet Notes vault in Setup to your Obsidian vault folder.\n"));
        s
    }

    #[test]
    fn pages_split_into_titled_sections() {
        let s = sample();
        assert_eq!(s[0].title, "Setup");
        assert_eq!(s[1].title, "Setup › 1. Model");
        assert!(s[1].text.contains("ollama pull"));
        assert_eq!(s[3].title, "Notes vault (Obsidian)");
    }

    #[test]
    fn questions_pick_matching_sections_and_small_talk_picks_none() {
        let s = sample();
        let picked = relevant_sections("How do I connect tableski?", &s, 3);
        assert_eq!(picked[0].title, "Setup › 3. tableski (optional)", "{picked:?}");
        assert_eq!(relevant_sections("Where do my obsidian notes go?", &s, 3)[0].title, "Notes vault (Obsidian)");
        assert!(relevant_sections("What is the capital of France?", &s, 3).is_empty());
        assert!(relevant_sections("hi", &s, 3).is_empty());
    }

    #[test]
    fn the_built_in_guide_is_embedded_and_answers_common_questions() {
        let all = guide_sections();
        assert!(all.len() > 10, "guide embedded: {} sections", all.len());
        let top = |q: &str| relevant_sections(q, &all, 1).first().map(|s| s.title.clone()).unwrap_or_default();
        assert!(top("Why is my run so slow?").starts_with("Troubleshooting"), "{}", top("Why is my run so slow?"));
        assert!(top("How do I switch on the morning brief schedule?").contains("Schedules"), "{}", top("How do I switch on the morning brief schedule?"));
        assert!(top("ollama has no models").contains("Setup") || top("ollama has no models").contains("Troubleshooting"));
        assert_eq!(top("Why did my last run fail?"), "Troubleshooting › A run failed");
        assert!(top("Where are my obsidian notes saved?").starts_with("Notes vault"), "{}", top("Where are my obsidian notes saved?"));
    }

    #[test]
    fn context_carries_snapshot_and_only_matching_guide() {
        let s = sample();
        let c = chat_context("- model: gemma4:e2b via ollama\n", "How do I connect tableski?", &s);
        assert!(c.contains("This kowalski right now") && c.contains("gemma4:e2b"));
        assert!(c.contains("Connect tableski signs you in"));
        let plain = chat_context("- model: x\n", "Write a haiku about the sea", &s);
        assert!(!plain.contains("From the kowalski guide"), "{plain}");
    }
}
