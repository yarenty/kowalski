---
name = "deliver"
kind = "deliver"
avatar = "deliver"
capability = "url-summarizer.deliver"
default_agent_id = "url-summarizer-deliver"
display_name = "Deliver"
description = "Compiles all individual summaries into the final HANDOFF file."
prompt_file = "prompts/deliver.md"
output = "HANDOFF.md"
context_paths = ["@artifact@"]
---

# Deliver

Compiles all individual summaries into the final HANDOFF file.
