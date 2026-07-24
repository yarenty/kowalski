---
name = "ingest"
kind = "ingest"
capability = "coder.ingest"
default_agent_id = "coder-ingest"
display_name = "Project intake"
description = "Captures operator project path and task specification."
prompt_file = "prompts/ingest.md"
output = "debug/raw/"
[[inputs]]
id = "project_path"
type = "path"
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
