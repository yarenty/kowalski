---
name = "summary"
kind = "process"
capability = "coding-assistant.process"
default_agent_id = "coding-assistant-summary"
display_name = "Summary"
description = "Executive summary of the run for the operator."
prompt_file = "prompts/summary.md"
output = "debug/stage-summary.md"
context_paths = ["@artifact@"]
---

# Summary
