---
name = "check-complete"
kind = "process"
capability = "coder.process"
default_agent_id = "coder-check-complete"
display_name = "Check completion"
description = "Acceptance checklist against the original task."
prompt_file = "prompts/check-complete.md"
output = "debug/stage-check-complete.md"
context_paths = ["@artifact@"]
---

# Check completion
