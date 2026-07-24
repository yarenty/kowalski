---
name = "summarize"
kind = "process"
avatar = "process"
capability = "url-summarizer.process"
default_agent_id = "url-summarizer-process"
display_name = "Summarize"
description = "Processes each ingested URL, extracts content, and generates a simple Markdown summary."
prompt_file = "prompts/summarize.md"
output = "debug/summaries/"
context_paths = ["@artifact@"]
---

# Summarize

Processes each ingested URL, extracts content, and generates a simple Markdown summary.
