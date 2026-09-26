---
name = "report"
kind = "xlsx_report"
capability = "spreadsheet-analyst.report"
default_agent_id = "spreadsheet-analyst-report"
display_name = "Build the workbook"
description = "report.xlsx: an index sheet plus one sheet per question, numbers as numbers."
output = "report.xlsx"
---

# Build the workbook

Deterministic: the workbook is built from the query engine's results, not from model text.
