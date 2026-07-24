---
name = "deliver"
kind = "deliver"
avatar = "deliver"
capability = "apps-builder-3000-horde.deliver"
default_agent_id = "apps-builder-3000-horde-deliver"
display_name = "Deliver"
description = "Confirms the final artifact path and creates a Handoff summary."
prompt_file = "prompts/deliver.md"
output = "HANDOFF.md"
context_paths = ["@artifact@"]
---

# Deliver

Confirms the final artifact path and creates a Handoff summary.
