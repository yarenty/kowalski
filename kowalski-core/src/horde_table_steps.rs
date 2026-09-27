//! Deterministic horde steps for data work, so numbers never pass through a model:
//!
//! - `sql_batch`: every fenced ```sql block in the previous step's artifact (a plan the model
//!   wrote, each block under a `###` heading) is run through a tool (default `query_sql`,
//!   e.g. tableski over MCP); the raw results go to a Markdown artifact plus a JSON sidecar.
//! - `table_profile`: every table the data tool lists is described from `list_tables`,
//!   `get_schema` and `column_statistics`, so the model that writes SQL starts from exact names.
//! - `xlsx_report`: the sidecar of the previous `sql_batch` becomes a workbook: an index sheet
//!   (question, SQL, rows) and one sheet per result, numbers typed as numbers.

use crate::error::KowalskiError;
use crate::horde_step::{StepContext, StepError, StepHandler, StepOutcome};
use crate::horde_stages::StageStatus;
use crate::tools::ToolInput;
use crate::tools::manager::ToolManager;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Most queries one `sql_batch` step runs.
const MAX_QUERIES: usize = 20;
/// Most tables one `table_profile` step describes.
const MAX_TABLES: usize = 20;
/// Advice for an operator whose workbook reads like a form rather than tables (a tip, never a stop).
pub const PREPARE_WORKBOOK_ADVICE: &str = "For better answers, prepare the workbook with each table on its own sheet, the column names in the first row and one record per row below them; no title, instruction or total rows above the header, no merged cells.";
/// Sample rows shown for a table with a header row.
const TABLE_SAMPLE_ROWS: usize = 5;
/// Rows read from a sheet without one (a form or report), whose content is its text.
const FORM_SAMPLE_ROWS: usize = 60;

/// One question of a plan: its heading and SQL, or the plan's reason for having no SQL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedQuery {
    /// The `###` heading above the block (the question it answers).
    pub title: String,
    /// The SQL text; empty when the plan says the data cannot answer the question.
    pub sql: String,
    /// The plan's prose under the heading (for an unanswerable question: why).
    pub note: String,
}

/// Fenced ```sql blocks with the nearest preceding heading as their title. A `###` (or deeper)
/// heading with no runnable block under it is kept with empty `sql`: a question the plan
/// declined, so it is reported rather than silently dropped.
pub fn parse_sql_plan(markdown: &str) -> Vec<PlannedQuery> {
    let mut out = Vec::new();
    // heading waiting for its block, whether it counts as a question, prose under it
    let mut heading: Option<(String, bool)> = None;
    let mut note = String::new();
    let mut in_block = false;
    let mut buf = String::new();
    let decline = |out: &mut Vec<PlannedQuery>, heading: &mut Option<(String, bool)>, note: &mut String| {
        if let Some((title, true)) = heading.take() {
            out.push(PlannedQuery { title, sql: String::new(), note: note.trim().to_string() });
        }
        note.clear();
    };
    for line in markdown.lines() {
        let t = line.trim_start();
        if in_block {
            if t.starts_with("```") {
                in_block = false;
                let sql = buf.trim().trim_end_matches(';').trim().to_string();
                let has_statement = sql.lines().any(|l| {
                    let l = l.trim();
                    !l.is_empty() && !l.starts_with("--")
                });
                if has_statement {
                    let n = out.len() + 1;
                    out.push(PlannedQuery {
                        title: heading.take().map(|(h, _)| h).unwrap_or_else(|| format!("Query {n}")),
                        sql,
                        note: note.trim().to_string(),
                    });
                    note.clear();
                }
                buf.clear();
            } else {
                buf.push_str(line);
                buf.push('\n');
            }
        } else if let Some(rest) = t.strip_prefix("```") {
            if rest.trim().eq_ignore_ascii_case("sql") {
                in_block = true;
            }
        } else if t.starts_with('#') {
            let h = t.trim_start_matches('#').trim();
            if !h.is_empty() {
                decline(&mut out, &mut heading, &mut note);
                let level = t.chars().take_while(|c| *c == '#').count();
                heading = Some((h.to_string(), level >= 3));
            }
        } else if heading.is_some() && !t.trim().is_empty() {
            note.push_str(t.trim());
            note.push(' ');
        }
    }
    decline(&mut out, &mut heading, &mut note);
    out
}

/// A result grid parsed out of a tool's text output (the Arrow pretty-print that tableski
/// returns: `+---+` rules and `| a | b |` rows, possibly inside a data frame).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResultGrid {
    /// Column names.
    pub columns: Vec<String>,
    /// Rows of cell text, one entry per column.
    pub rows: Vec<Vec<String>>,
}

/// Parse the first `| … |` table in `text`: the first row is the header.
pub fn parse_result_grid(text: &str) -> Option<ResultGrid> {
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('|') && l.ends_with('|') && l.len() > 1);
    let split = |l: &str| -> Vec<String> { l[1..l.len() - 1].split('|').map(|c| c.trim().to_string()).collect() };
    let columns = split(lines.next()?);
    let rows = lines.map(split).filter(|r| r.len() == columns.len()).collect();
    Some(ResultGrid { columns, rows })
}

/// What one query produced, as written to the JSON sidecar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResult {
    /// Heading of the query.
    pub title: String,
    /// SQL that ran.
    pub sql: String,
    /// Parsed grid when the tool answered with one.
    pub grid: Option<ResultGrid>,
    /// Raw tool text (framing included).
    pub raw: String,
    /// The tool's error, when it failed.
    pub error: Option<String>,
}

fn tool_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|i| i.get("text").and_then(Value::as_str).or_else(|| i.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(o) => match o.get("content").or_else(|| o.get("text")) {
            Some(inner) => tool_text(inner),
            None => v.to_string(),
        },
        other => other.to_string(),
    }
}

fn sidecar(path: &Path) -> PathBuf {
    path.with_extension("json")
}

fn artifact_path(ctx: &StepContext<'_>) -> Result<PathBuf, StepError> {
    let rel = ctx.step.output.as_deref().ok_or_else(|| {
        KowalskiError::Validation(format!("{} stage `{}` missing `output`", ctx.step.kind, ctx.step.name))
    })?;
    let out = ctx.workdir.join(rel);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| KowalskiError::Validation(e.to_string()))?;
    }
    Ok(out)
}

fn previous(ctx: &StepContext<'_>) -> Result<PathBuf, StepError> {
    ctx.previous_artifact
        .map(Path::to_path_buf)
        .ok_or_else(|| KowalskiError::Validation(format!("stage `{}` needs the previous step's output", ctx.step.name)))
}

/// `sql_batch`: run the plan's SQL blocks through a tool.
pub struct SqlBatchStepHandler {
    tools: ToolManager,
}

impl SqlBatchStepHandler {
    /// Handler calling tools from `tools` (the horde agent's tool manager, MCP proxies included).
    pub fn new(tools: ToolManager) -> Self {
        Self { tools }
    }
}

#[async_trait]
impl StepHandler for SqlBatchStepHandler {
    fn kind(&self) -> &'static str {
        "sql_batch"
    }

    async fn execute(&self, ctx: &StepContext<'_>) -> Result<StepOutcome, StepError> {
        let plan_path = previous(ctx)?;
        let plan = std::fs::read_to_string(&plan_path)
            .map_err(|e| KowalskiError::Validation(format!("read plan {}: {e}", plan_path.display())))?;
        let queries = parse_sql_plan(&plan);
        if queries.iter().all(|q| q.sql.is_empty()) {
            return Err(KowalskiError::Validation("the plan contains no ```sql blocks".into()));
        }
        let tool = ctx.step.tool_ids.first().map(String::as_str).unwrap_or("query_sql").to_string();
        if self.tools.get(&tool).is_none() {
            return Err(KowalskiError::Validation(format!(
                "tool `{tool}` is not available; connect tableski (Setup) or add the MCP server that provides it"
            )));
        }
        let out = artifact_path(ctx)?;
        let mut results = Vec::new();
        let mut md = String::from("# Query results\n\nComputed by the data tool; the model did not produce these numbers.\n");
        for (i, q) in queries.into_iter().take(MAX_QUERIES).enumerate() {
            if ctx.cancel.is_cancelled() {
                return Err(KowalskiError::Validation(format!("step `{}` cancelled", ctx.step.name)));
            }
            if q.sql.is_empty() {
                let why = if q.note.is_empty() { "the plan wrote no query for it".to_string() } else { q.note };
                md.push_str(&format!("\n## {}\n\n**Not answerable from these tables:** {why}\n", q.title));
                results.push(QueryResult { title: q.title, sql: q.sql, grid: None, raw: String::new(), error: Some(format!("Not answerable from these tables: {why}")) });
                continue;
            }
            ctx.events.message(&format!("Running query {}: {}", i + 1, q.title)).await;
            let (mut raw, mut error) = self.run_query(&tool, &q.sql).await;
            let mut q = q;
            // a model that copied a sheet name ("Sheet1") for the table (sheet1): one retry with
            // the registered name when only the spelling's case differs
            if let Some(e) = error.as_deref()
                && let Some(fixed) = self.fix_table_case(&q.sql, e).await
            {
                ctx.events.message(&format!("Retrying {} with the registered table name", q.title)).await;
                let (r2, e2) = self.run_query(&tool, &fixed).await;
                if e2.is_none() {
                    q.note = format!("{} (table name corrected)", q.note).trim().to_string();
                    q.sql = fixed;
                    raw = r2;
                    error = None;
                }
            }
            let grid = error.is_none().then(|| parse_result_grid(&raw)).flatten();
            md.push_str(&format!("\n## {}\n\n```sql\n{}\n```\n\n", q.title, q.sql));
            match (&error, &grid) {
                (Some(e), _) => md.push_str(&format!("**Error:** {e}\n")),
                (None, Some(g)) => {
                    md.push_str(&format!("{} rows.\n\n| {} |\n|{}|\n", g.rows.len(), g.columns.join(" | "), g.columns.iter().map(|_| "---").collect::<Vec<_>>().join("|")));
                    for r in g.rows.iter().take(50) {
                        md.push_str(&format!("| {} |\n", r.join(" | ")));
                    }
                    if g.rows.len() > 50 {
                        md.push_str(&format!("\n… {} more rows in the report workbook.\n", g.rows.len() - 50));
                    }
                }
                (None, None) => md.push_str(&format!("```text\n{}\n```\n", raw.trim())),
            }
            results.push(QueryResult { title: q.title, sql: q.sql, grid, raw, error });
        }
        std::fs::write(&out, &md).map_err(|e| KowalskiError::Validation(e.to_string()))?;
        std::fs::write(sidecar(&out), serde_json::to_vec_pretty(&results).unwrap_or_default())
            .map_err(|e| KowalskiError::Validation(e.to_string()))?;
        let failed = results.iter().filter(|r| r.error.is_some()).count();
        Ok(StepOutcome::Completed {
            summary: format!("{} queries run, {failed} failed: {}", results.len(), out.display()),
            artifact: Some(out),
            status: if failed == results.len() { StageStatus::Fail } else { StageStatus::Pass },
        })
    }
}

/// The JSON object inside a framed tool answer (text before the first `{` and after the last `}`
/// is framing).
fn framed_json(text: &str) -> Option<Value> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    serde_json::from_str(text.get(start..=end)?).ok()
}

/// `table_profile`: describe every table the data tool exposes, without a model.
pub struct TableProfileStepHandler {
    tools: ToolManager,
}

impl TableProfileStepHandler {
    /// Handler calling `list_tables`, `get_schema` and `column_statistics` from `tools`.
    pub fn new(tools: ToolManager) -> Self {
        Self { tools }
    }

    async fn call(&self, tool: &str, args: Value) -> Result<String, String> {
        let input = ToolInput::new(tool.into(), String::new(), args);
        self.tools.execute(tool, input).await.map(|o| tool_text(&o.result)).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl StepHandler for TableProfileStepHandler {
    fn kind(&self) -> &'static str {
        "table_profile"
    }

    async fn execute(&self, ctx: &StepContext<'_>) -> Result<StepOutcome, StepError> {
        if self.tools.get("list_tables").is_none() {
            return Err(KowalskiError::Validation(
                "tool `list_tables` is not available; connect tableski (Setup) or add the MCP server that provides it".into(),
            ));
        }
        let listing = self.call("list_tables", json!({})).await.map_err(KowalskiError::Validation)?;
        let tables: Vec<Value> = framed_json(&listing)
            .and_then(|v| v.get("tables").and_then(Value::as_array).cloned())
            .unwrap_or_default();
        if tables.is_empty() {
            return Err(KowalskiError::Validation(format!("the data tool lists no tables: {}", listing.trim())));
        }
        let out = artifact_path(ctx)?;
        let mut md = String::from("# Data profile\n\nRead from the data tool; use these table and column names exactly.\n");
        // sheets that turned out to be forms or reports rather than tables (no header row)
        let mut forms: Vec<String> = Vec::new();
        for t in tables.iter().take(MAX_TABLES) {
            if ctx.cancel.is_cancelled() {
                return Err(KowalskiError::Validation(format!("step `{}` cancelled", ctx.step.name)));
            }
            let Some(name) = t.get("name").and_then(Value::as_str) else { continue };
            ctx.events.message(&format!("Profiling table {name}")).await;
            md.push_str(&format!(
                "\n## Table `{name}`\n\n- SQL name: `{name}` (write it exactly so; a sheet name is not a table name)\n"
            ));
            for (key, label) in [("source", "Source"), ("sheet", "Sheet"), ("rows", "Rows"), ("columns", "Columns")] {
                match t.get(key) {
                    Some(Value::String(v)) => md.push_str(&format!("- {label}: {v}\n")),
                    Some(v @ Value::Number(_)) => md.push_str(&format!("- {label}: {v}\n")),
                    _ => {}
                }
            }
            let mut form_like = false;
            match self.call("get_schema", json!({ "table": name })).await {
                Ok(text) => {
                    let cols = framed_json(&text).and_then(|v| v.get("columns").and_then(Value::as_array).cloned()).unwrap_or_default();
                    // no header row found: tableski names the columns col_1, col_2, …
                    form_like = !cols.is_empty()
                        && cols.iter().all(|c| {
                            c.get("name")
                                .and_then(Value::as_str)
                                .and_then(|n| n.strip_prefix("col_"))
                                .is_some_and(|d| d.chars().all(|ch| ch.is_ascii_digit()))
                        });
                    md.push_str("\n| column | type | nullable |\n|---|---|---|\n");
                    for c in cols {
                        md.push_str(&format!(
                            "| `{}` | {} | {} |\n",
                            c.get("name").and_then(Value::as_str).unwrap_or("?"),
                            c.get("data_type").and_then(Value::as_str).unwrap_or("?"),
                            c.get("nullable").and_then(Value::as_bool).unwrap_or(true),
                        ));
                    }
                }
                Err(e) => md.push_str(&format!("\nSchema unavailable: {e}\n")),
            }
            if self.tools.get("query_sql").is_some() {
                let limit = if form_like { FORM_SAMPLE_ROWS } else { TABLE_SAMPLE_ROWS };
                let sql = format!("SELECT * FROM {name} LIMIT {limit}");
                if let Ok(text) = self.call("query_sql", json!({ "sql": sql })).await
                    && let Some(g) = parse_result_grid(&text)
                {
                    if form_like {
                        md.push_str("\nNo header row: this sheet looks like a form or report, not a table. Its text is in the rows below (empty cells left out); answer questions about what it contains from these rows, and find values with `WHERE col_1 LIKE '%…%'`.\n\n");
                        for row in g.rows.iter() {
                            let cells: Vec<String> = g
                                .columns
                                .iter()
                                .zip(row)
                                .filter(|(_, v)| !v.trim().is_empty())
                                .map(|(c, v)| format!("{c}: {}", v.trim()))
                                .collect();
                            if !cells.is_empty() {
                                let line = cells.join(" · ");
                                md.push_str(&format!("- {}\n", crate::markdown_pipeline::cap_chars(&line, 220).trim_end()));
                            }
                        }
                    } else {
                        md.push_str(&format!("\nFirst rows:\n\n| {} |\n|{}|\n", g.columns.join(" | "), g.columns.iter().map(|_| "---").collect::<Vec<_>>().join("|")));
                        for r in &g.rows {
                            md.push_str(&format!("| {} |\n", r.join(" | ")));
                        }
                    }
                }
            }
            if form_like {
                let origin = match (t.get("sheet").and_then(Value::as_str), t.get("source").and_then(Value::as_str)) {
                    (Some(sheet), Some(src)) => format!("sheet `{sheet}` of `{}`", src.rsplit('/').next().unwrap_or(src)),
                    (None, Some(src)) => format!("`{}`", src.rsplit('/').next().unwrap_or(src)),
                    _ => format!("table `{name}`"),
                };
                ctx.events.message(&format!("{origin} has no header row: reading it as a form (a tip on preparing it comes with the answers)")).await;
                forms.push(origin);
            }
            if self.tools.get("column_statistics").is_some() && !form_like {
                match self.call("column_statistics", json!({ "table": name })).await {
                    Ok(text) => match parse_result_grid(&text) {
                        Some(g) => {
                            md.push_str(&format!("\nStatistics:\n\n| {} |\n|{}|\n", g.columns.join(" | "), g.columns.iter().map(|_| "---").collect::<Vec<_>>().join("|")));
                            for r in &g.rows {
                                md.push_str(&format!("| {} |\n", r.join(" | ")));
                            }
                        }
                        None => md.push_str(&format!("\nStatistics:\n\n```text\n{}\n```\n", text.trim())),
                    },
                    Err(e) => md.push_str(&format!("\nStatistics unavailable: {e}\n")),
                }
            }
        }
        if !forms.is_empty() {
            let warning = format!(
                "\n## Note: a sheet without a header row\n\n{} {} no header row, so {} like a form or report rather than a table. It is still used: its text rows are listed below and answers come from them, but a table answers better. {}\n",
                forms.join(", "),
                if forms.len() == 1 { "has" } else { "have" },
                if forms.len() == 1 { "it reads" } else { "they read" },
                PREPARE_WORKBOOK_ADVICE,
            );
            md.insert_str(md.find('\n').map(|i| i + 1).unwrap_or(0), &warning);
        }
        std::fs::write(&out, &md).map_err(|e| KowalskiError::Validation(e.to_string()))?;
        Ok(StepOutcome::Completed {
            summary: format!("{} tables profiled: {}", tables.len().min(MAX_TABLES), out.display()),
            artifact: Some(out),
            status: StageStatus::Pass,
        })
    }
}

impl SqlBatchStepHandler {
    async fn run_query(&self, tool: &str, sql: &str) -> (String, Option<String>) {
        let input = ToolInput::new("query".into(), String::new(), json!({ "sql": sql }));
        match self.tools.execute(tool, input).await {
            Ok(o) => (tool_text(&o.result), None),
            Err(e) => (String::new(), Some(e.to_string())),
        }
    }

    /// `sql` with a table reference respelled to the registered name, when the data tool refused
    /// it as unregistered and a registered table differs from it only in case (or quoting).
    async fn fix_table_case(&self, sql: &str, error: &str) -> Option<String> {
        let rest = error.split("`").nth(1)?; // the refused name, as the tool quotes it
        if !error.contains("is not a registered table") {
            return None;
        }
        let bare = rest.trim_matches('"');
        self.tools.get("list_tables")?;
        let listing = self
            .tools
            .execute("list_tables", ToolInput::new("list_tables".into(), String::new(), json!({})))
            .await
            .ok()
            .map(|o| tool_text(&o.result))?;
        let tables = framed_json(&listing)?.get("tables")?.as_array()?.clone();
        let registered = tables
            .iter()
            .filter_map(|t| t.get("name").and_then(Value::as_str))
            .find(|n| n.eq_ignore_ascii_case(bare) && *n != bare)?
            .to_string();
        let fixed = sql.replace(&format!("\"{bare}\""), &registered).replace(bare, &registered);
        (fixed != sql).then_some(fixed)
    }
}

/// Excel sheet name: at most 31 characters, none of `[]:*?/\`, unique within the workbook.
fn sheet_name(title: &str, taken: &mut Vec<String>) -> String {
    let clean: String = title
        .chars()
        .map(|c| if "[]:*?/\\".contains(c) { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let base: String = clean.chars().take(28).collect();
    let base = if base.is_empty() { "Result".to_string() } else { base };
    let mut name = base.clone();
    let mut n = 2;
    while taken.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
        name = format!("{base} {n}");
        n += 1;
    }
    taken.push(name.clone());
    name
}

/// Write `results` to an xlsx workbook at `path`.
pub fn write_report_workbook(results: &[QueryResult], path: &Path) -> Result<(), String> {
    use rust_xlsxwriter::{Format, Workbook};
    let mut wb = Workbook::new();
    let bold = Format::new().set_bold();
    let mut taken = vec!["Answers".to_string()];
    let names: Vec<String> = results.iter().map(|r| sheet_name(&r.title, &mut taken)).collect();
    {
        let idx = wb.add_worksheet();
        idx.set_name("Answers").map_err(|e| e.to_string())?;
        for (c, h) in ["Question", "Sheet", "Rows", "SQL", "Error"].iter().enumerate() {
            idx.write_string_with_format(0, c as u16, *h, &bold).map_err(|e| e.to_string())?;
        }
        for (i, r) in results.iter().enumerate() {
            let row = (i + 1) as u32;
            idx.write_string(row, 0, &r.title).map_err(|e| e.to_string())?;
            idx.write_string(row, 1, &names[i]).map_err(|e| e.to_string())?;
            idx.write_number(row, 2, r.grid.as_ref().map(|g| g.rows.len()).unwrap_or(0) as f64).map_err(|e| e.to_string())?;
            idx.write_string(row, 3, &r.sql).map_err(|e| e.to_string())?;
            if let Some(e) = &r.error {
                idx.write_string(row, 4, e).map_err(|e| e.to_string())?;
            }
        }
        idx.set_column_width(0, 48).map_err(|e| e.to_string())?;
        idx.set_column_width(3, 80).map_err(|e| e.to_string())?;
    }
    for (i, r) in results.iter().enumerate() {
        let ws = wb.add_worksheet();
        ws.set_name(&names[i]).map_err(|e| e.to_string())?;
        let Some(g) = &r.grid else {
            ws.write_string(0, 0, r.error.as_deref().unwrap_or("no table in the result")).map_err(|e| e.to_string())?;
            continue;
        };
        for (c, h) in g.columns.iter().enumerate() {
            ws.write_string_with_format(0, c as u16, h, &bold).map_err(|e| e.to_string())?;
        }
        for (ri, row) in g.rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                let (rr, cc) = ((ri + 1) as u32, c as u16);
                match cell.replace(',', "").parse::<f64>() {
                    Ok(n) if !cell.is_empty() && n.is_finite() => ws.write_number(rr, cc, n),
                    _ => ws.write_string(rr, cc, cell),
                }
                .map_err(|e| e.to_string())?;
            }
        }
        ws.autofit();
    }
    wb.save(path).map_err(|e| e.to_string())
}

/// `xlsx_report`: the previous `sql_batch` results as a workbook.
pub struct XlsxReportStepHandler;

#[async_trait]
impl StepHandler for XlsxReportStepHandler {
    fn kind(&self) -> &'static str {
        "xlsx_report"
    }

    async fn execute(&self, ctx: &StepContext<'_>) -> Result<StepOutcome, StepError> {
        let prev = previous(ctx)?;
        let raw = std::fs::read(sidecar(&prev)).map_err(|e| {
            KowalskiError::Validation(format!("xlsx_report needs a sql_batch step before it ({}: {e})", sidecar(&prev).display()))
        })?;
        let results: Vec<QueryResult> = serde_json::from_slice(&raw).map_err(|e| KowalskiError::Validation(e.to_string()))?;
        let out = artifact_path(ctx)?;
        let target = out.clone();
        tokio::task::spawn_blocking(move || write_report_workbook(&results, &target))
            .await
            .map_err(|e| KowalskiError::Validation(e.to_string()))?
            .map_err(KowalskiError::Validation)?;
        ctx.events.message(&format!("Report workbook written: {}", out.display())).await;
        Ok(StepOutcome::Completed {
            summary: format!("report workbook: {}", out.display()),
            artifact: Some(prev),
            status: StageStatus::Pass,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_blocks_take_their_headings() {
        let plan = "# Plan\n\n### Q1: Who spent the most?\nWe sum orders.\n```sql\nSELECT name, SUM(amount) AS total FROM orders GROUP BY name;\n```\n\n### Q2: How many people?\n```SQL\nSELECT count(*) FROM people\n```\n```python\nprint(1)\n```\n```sql\nSELECT 1\n```\n### Q4: Not answerable\n```sql\n-- No SQL query\n```\n";
        let q = parse_sql_plan(plan);
        assert_eq!(q.len(), 4, "{q:?}");
        assert_eq!(q[0].title, "Q1: Who spent the most?");
        assert_eq!(q[0].note, "We sum orders.");
        assert_eq!(q[0].sql, "SELECT name, SUM(amount) AS total FROM orders GROUP BY name");
        assert_eq!(q[1].title, "Q2: How many people?");
        assert_eq!(q[2].title, "Query 3", "no heading left for the third block");
        assert_eq!(q[3].title, "Q4: Not answerable");
        assert!(q[3].sql.is_empty(), "a comment-only block is no query");
    }

    #[test]
    fn grid_parses_arrow_pretty_output_inside_a_frame() {
        let text = "The following is a computed result...\n----- BEGIN_DATA_x -----\n+-------+-------+\n| name  | total |\n+-------+-------+\n| ada   | 150.5 |\n| linus | 99.99 |\n+-------+-------+\n----- END_DATA_x -----";
        let g = parse_result_grid(text).unwrap();
        assert_eq!(g.columns, vec!["name", "total"]);
        assert_eq!(g.rows, vec![vec!["ada", "150.5"], vec!["linus", "99.99"]]);
        assert!(parse_result_grid("no table here").is_none());
    }

    #[test]
    fn sheet_names_are_valid_and_unique() {
        let mut taken = vec!["Answers".to_string()];
        let a = sheet_name("Q1: Which customers [top 10] / region?", &mut taken);
        assert!(a.len() <= 31 && !a.contains(':') && !a.contains('[') && !a.contains('/'), "{a}");
        let b = sheet_name("Q1: Which customers [top 10] / region?", &mut taken);
        assert_ne!(a, b);
        assert_eq!(sheet_name("answers", &mut taken), "answers 2");
    }

    #[test]
    fn workbook_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("report.xlsx");
        let results = vec![
            QueryResult { title: "Q1: totals".into(), sql: "SELECT 1".into(), grid: Some(ResultGrid { columns: vec!["name".into(), "total".into()], rows: vec![vec!["ada".into(), "1,150.5".into()]] }), raw: String::new(), error: None },
            QueryResult { title: "Q2: broken".into(), sql: "SELECT nope".into(), grid: None, raw: String::new(), error: Some("no such column".into()) },
        ];
        write_report_workbook(&results, &p).unwrap();
        let bytes = std::fs::read(&p).unwrap();
        assert!(bytes.starts_with(b"PK"), "an xlsx is a zip");
        assert!(bytes.len() > 2000);
    }
}
