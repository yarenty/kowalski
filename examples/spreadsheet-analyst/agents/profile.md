---
name = "profile"
kind = "table_profile"
capability = "spreadsheet-analyst.profile"
default_agent_id = "spreadsheet-analyst-profile"
display_name = "Profile the tables"
description = "Reads each table's columns, types and statistics through tableski, without a model."
tool_ids = ["list_tables", "get_schema", "column_statistics"]
output = "debug/profile.md"
---

# Profile the tables

Calls tableski's read-only tools directly, so the SQL writer starts from exact table and column
names even on a small local model.
