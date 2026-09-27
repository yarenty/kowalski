---
id = "spreadsheet-analyst"
display_name = "Spreadsheet Analyst"
description = "Ask questions about your spreadsheets in plain words: the tables are profiled, each question becomes SQL run by tableski, and you get answers plus a report workbook. Numbers come from the query engine, never from the model."
capability_prefix = "spreadsheet-analyst"
category = "spreadsheets"
icon = "table"
featured = true
pipeline = ["ingest", "profile", "plan", "run", "report", "deliver"]
default_question = "Answer the questions about my spreadsheets."
default_topic = "federation"
artifacts_root = "."
workdir = "output"
delivery_title = "Your answers"
delivery_note = "Open **`workdir/HANDOFF.md`** for the answers and **`workdir/report.xlsx`** for every result table. The SQL behind each answer is in `debug/results.md`."
delivery_root_rel = "HANDOFF.md"
delivery_summary_note = "Answers to your spreadsheet questions, computed with SQL by tableski, plus a report workbook."
prompt_tip = "Add your workbook under Your workbooks (tableski must be connected in Setup), then list one question per line."
---

# Spreadsheet Analyst

Needs **tableski** connected (Setup → Connect tableski, or a `[[mcp.servers]]` entry named for any
server offering `list_tables`, `get_schema`, `column_statistics` and `query_sql`). Add your
workbook or CSV under **Your workbooks** on this horde's page (it goes to your tableski account);
every sheet becomes a table.

## Steps (penguins)

- `ingest` (ingest): your questions, one per line, and optionally which tables to use.
- `profile` (table_profile, no model): reads every table's columns, types and statistics.
- `plan` (process): writes one read-only SQL query per question.
- `run` (sql_batch): runs each query through tableski's `query_sql`; results are recorded exactly as returned.
- `report` (xlsx_report): builds `report.xlsx`, one sheet per question plus an index.
- `deliver` (deliver): writes `HANDOFF.md`, answering each question with the numbers from the results.

The model never writes a number into the report: it writes SQL, the query engine computes, and the
workbook is built from the engine's output.
