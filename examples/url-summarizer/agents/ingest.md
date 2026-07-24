---
name = "ingest"
kind = "ingest"
avatar = "ingest"
capability = "url-summarizer.ingest"
default_agent_id = "url-summarizer-ingest"
display_name = "Ingest"
description = "Receives the list of URLs provided by the operator via input form."
prompt_file = "prompts/ingest.md"
output = "debug/raw/"
[[inputs]]
id = "urls"
type = "text"
label = "URLs to process"
required = true
---

# Ingest

Receives the list of URLs provided by the operator via input form.
