---
name = "ingest"
kind = "ingest"
avatar = "ingest"
capability = "apps-builder-3000-horde.ingest"
default_agent_id = "apps-builder-3000-horde-ingest"
display_name = "Ingest"
description = "Gathers the required application name from the operator."
prompt_file = "prompts/ingest.md"
output = "debug/raw/app_name.txt"
[[inputs]]
id = "app_name"
type = "text"
label = "Application Name"
required = true
---

# Ingest

Gathers the required application name from the operator.
