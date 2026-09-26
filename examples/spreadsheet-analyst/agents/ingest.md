---
name = "ingest"
kind = "ingest"
capability = "spreadsheet-analyst.ingest"
default_agent_id = "spreadsheet-analyst-ingest"
display_name = "Your questions"
description = "Your questions about the spreadsheets, and optionally which tables to use."
prompt_file = "prompts/ingest.md"
output = "debug/raw/"
[[inputs]]
id = "questions"
type = "textarea"
label = "Questions, one per line"
required = true
placeholder = "Which customers bought the most last quarter?\nHow did monthly revenue change this year?"
[[inputs]]
id = "tables"
type = "text"
label = "Tables to use (optional, comma separated; empty = all)"
required = false
placeholder = "orders, customers"
---

# Your questions

The operator's questions and table choice, recorded for the next steps.
