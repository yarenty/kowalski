---
name = "deliver"
kind = "deliver"
capability = "coder.deliver"
default_agent_id = "coder-deliver"
display_name = "Deliver"
description = "Final operator handoff markdown."
prompt_file = "prompts/deliver.md"
output = "HANDOFF.md"
context_paths = ["@step:summary@", "@step:adjust@", "@step:dev-2@", "@step:test-verify@", "@step:review@"]
---

# Deliver
