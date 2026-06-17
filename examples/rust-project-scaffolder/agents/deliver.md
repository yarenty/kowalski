---
name = "deliver"
kind = "final"
capability = "rust-project-scaffolder.final"
default_agent_id = "rust-project-scaffolder-final"
display_name = "Final Output Delivery"
description = "Compiles and delivers the final structured output."
prompt_file = "prompts/deliver.md"
output = "HANDOFF.md"
context_paths = ["@artifact@"]
---

# Final Output Delivery

Compiles and delivers the final structured output.
