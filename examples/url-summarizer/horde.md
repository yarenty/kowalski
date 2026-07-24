---
id = "url-summarizer"
display_name = "URL Summarizer Horde"
description = "Ingests URLs from the operator, summarizes their content into Markdown, and compiles a final handoff file."
capability_prefix = "url-summarizer"
pipeline = ["ingest", "summarize", "deliver"]
default_question = "What should we do with the latest output?"
default_topic = "federation"
artifacts_root = "."
workdir = "output"
delivery_title = "Delivery"
delivery_note = "When the run finishes, open **`workdir/HANDOFF.md`**. Intermediates live under **`workdir/debug/`** per agent `output` paths."
delivery_root_rel = "HANDOFF.md"
delivery_summary_note = "Ingests URLs from the operator, summarizes their content into Markdown, and compiles a final handoff file."
---

# URL Summarizer Horde

Ingests URLs from the operator, summarizes their content into Markdown, and compiles a final handoff file.

## Sub-agents (penguins)

- `ingest` (ingest): Receives the list of URLs provided by the operator via input form.
- `summarize` (process): Processes each ingested URL, extracts content, and generates a simple Markdown summary.
- `deliver` (deliver): Compiles all individual summaries into the final HANDOFF file.

## Orchestration model

Linear pipeline:

```
ingest -> summarize -> deliver
```
