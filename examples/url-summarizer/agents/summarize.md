---
name = "summarize"
kind = "process"
avatar = "process"
capability = "url-summarizer.process"
default_agent_id = "url-summarizer-process"
display_name = "Summarize"
description = "Summarizes each fetched page and writes the hand-off file."
prompt_file = "prompts/summarize.md"
output = "HANDOFF.md"
context_paths = ["@artifact@"]
---

# Summarize

Summarizes each fetched page and writes the hand-off file.
