---
name = "run"
kind = "sql_batch"
capability = "spreadsheet-analyst.run"
default_agent_id = "spreadsheet-analyst-run"
display_name = "Run the queries"
description = "Runs every query of the plan through tableski's query_sql and records the results exactly as returned."
tool_ids = ["query_sql"]
output = "debug/results.md"
---

# Run the queries

Deterministic: no model involved. Each ```sql block of the plan is executed by the tool; the raw
results are kept next to the Markdown in `results.json` for the report.
