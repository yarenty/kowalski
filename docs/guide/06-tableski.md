# tableski: your spreadsheets as SQL

[tableski](https://tableski.io) turns every sheet of a workbook into a SQL table an AI can query.
The Spreadsheet analyst uses it; agents in Chat can use its tools too.

- **Hosted, tableski.io:** connect it in **Setup** (one sign-in). Then drop a workbook on the
  **Workbooks** card of the Spreadsheet analyst's page: it uploads to your account and lists the
  tables it made.
- **Your own tableski:** run `tableski --file report.xlsx` and point kowalski's config at it
  (`[[mcp.servers]] url = "http://127.0.0.1:8080/"`). Nothing leaves your machine. It serves the
  files it was started with; restart it with another `--file` to add one. Kowalski picks it up
  even if you start it after kowalski.

Workbooks work best with one table per sheet, column names in the first row, and no title rows or
merged cells. A sheet without a header row is still read, as labelled rows, and the answers end
with a tip.
