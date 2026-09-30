---
name = "ingest"
kind = "ingest"
capability = "morning-brief.ingest"
default_agent_id = "morning-brief-ingest"
display_name = "Fetch your pages"
description = "Fetches every page you listed."
prompt_file = "prompts/ingest.md"
output = "debug/raw/"
[[inputs]]
id = "sources"
type = "textarea"
label = "Pages to read, one per line"
required = true
default = "https://github.com/trending/rust\nhttps://tldr.tech/\nhttps://rss.arxiv.org/rss/cs.AI\nhttps://news.ycombinator.com/"
placeholder = "One link per line"
[[inputs]]
id = "focus"
type = "text"
label = "What you care about"
required = false
default = "Rust, AI agents and LLM tooling, data engineering, and research worth knowing about."
placeholder = "Supplier prices, payment regulation, competitors' launches"
---

# Fetch your pages

Every listed page, fetched and stored as Markdown for the brief.
