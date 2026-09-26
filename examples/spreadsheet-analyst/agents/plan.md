---
name = "plan"
kind = "process"
capability = "spreadsheet-analyst.plan"
default_agent_id = "spreadsheet-analyst-plan"
display_name = "Write the SQL"
description = "One read-only SQL query per question, using the profile's exact names."
prompt_file = "prompts/plan.md"
output = "debug/plan.md"
context_paths = ["@step:ingest@", "@step:profile@"]
---

# Write the SQL

Turns each question into one SELECT query. The next step runs them; this step never sees results.
