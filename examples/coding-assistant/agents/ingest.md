---
name = "ingest"
kind = "ingest"
capability = "coding-assistant.ingest"
default_agent_id = "coding-assistant-ingest"
display_name = "Project intake"
description = "Captures operator project path and task specification."
prompt_file = "prompts/ingest.md"
output = "debug/raw/"
[[inputs]]
id = "project_path"
type = "text"
label = "Project path (local folder or repo root)"
required = true
placeholder = "/path/to/your/project"
[[inputs]]
id = "task_spec"
type = "textarea"
label = "Task specification"
required = true
placeholder = "What should be done? Constraints, files, acceptance criteria…"
---

# Project intake
