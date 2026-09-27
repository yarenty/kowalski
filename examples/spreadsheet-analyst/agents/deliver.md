---
name = "deliver"
kind = "deliver"
capability = "spreadsheet-analyst.deliver"
default_agent_id = "spreadsheet-analyst-deliver"
display_name = "Write the answers"
description = "HANDOFF.md: each question answered in plain words, quoting the computed results."
prompt_file = "prompts/deliver.md"
output = "HANDOFF.md"
context_paths = ["@artifact@", "@step:ingest@", "@step:profile@"]
---

# Write the answers

Writes the reader-facing summary from the recorded results.
