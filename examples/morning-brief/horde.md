---
id = "morning-brief"
display_name = "Morning Brief"
description = "A one-page brief from the pages you follow: what changed, why it matters to you, and the links. Run it now, or switch on the weekday 7:00 schedule in the Hordes screen."
capability_prefix = "morning-brief"
category = "web"
icon = "sunrise"
featured = true
pipeline = ["ingest", "brief"]
default_question = "Write my morning brief."
default_topic = "federation"
artifacts_root = "."
workdir = "output"
delivery_title = "Your morning brief"
delivery_note = "Open **`workdir/BRIEF.md`**. The fetched pages are kept in `debug/raw/`, one file per run."
delivery_root_rel = "BRIEF.md"
delivery_summary_note = "A one-page brief from the pages you follow."
prompt_tip = "List the pages you check every morning (news, a competitor's prices, a supplier's announcements), one per line, and say what you care about."

[[triggers]]
cron = "0 7 * * 1-5"
enabled = false
input = { sources = "https://github.com/trending/rust\nhttps://tldr.tech/\nhttps://arxiv.org/list/cs/recent\nhttps://news.ycombinator.com/", focus = "Rust, AI agents and LLM tooling, data engineering, and research worth knowing about." }
---

# Morning Brief

Reads the pages you list and writes `BRIEF.md`: the few items worth your time, why each matters
for what you told it you care about, and a link to each.

## Steps (penguins)

- `ingest` (ingest): fetches every page you listed (public addresses only) into `debug/raw/`.
- `brief` (process): writes the one-page brief from what was fetched, and nothing else.

## Schedule

The `[[triggers]]` entry runs it at 7:00 on weekdays with the pages above. It ships switched off:
turn it on in the Hordes screen. To follow your own pages on the schedule, copy this folder into your
hordes folder, change `input`, and the copy replaces the built-in one.
