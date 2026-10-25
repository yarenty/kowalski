---
id = "url-summarizer"
display_name = "URL Summarizer Horde"
description = "Reads the pages at the URLs you give it and writes a short Markdown summary of each into one hand-off file."
capability_prefix = "url-summarizer"
category = "web"
icon = "link"
pipeline = ["ingest", "summarize"]
default_question = "What should we do with the latest output?"
default_topic = "federation"
artifacts_root = "."
workdir = "output"
delivery_title = "Delivery"
delivery_note = "When the run finishes, open **`workdir/HANDOFF.md`**. The fetched pages live under **`workdir/debug/raw/`**."
delivery_root_rel = "HANDOFF.md"
delivery_summary_note = "Reads the pages at the URLs you give it and writes a short Markdown summary of each into one hand-off file."
---

# URL Summarizer Horde

Reads the pages at the URLs you give it and writes a short Markdown summary of each into one hand-off file.

## Sub-agents (penguins)

- `ingest` (ingest): Receives the list of URLs provided by the operator via input form.
- `summarize` (process): Summarizes each fetched page and writes the hand-off file.

## Orchestration model

Linear pipeline:

```
ingest -> summarize
```
