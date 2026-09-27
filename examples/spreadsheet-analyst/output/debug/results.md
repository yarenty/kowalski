# Query results

Computed by the data tool; the model did not produce these numbers.

## Q1: what is all about

```sql
SELECT
  SUM(col_1)
FROM "Sheet1"
```

**Error:** Tool execution error: MCP error tableski: {"code":-32000,"message":"rejected: `\"Sheet1\"` is not a registered table (see list_tables)"}

## Q2: give me list of fields

```sql
SELECT
  column
FROM information_schema.columns
WHERE
  table_name IN ("Sheet1", "bars_1d", "bars_1h", "bars_4h", "indicators_1d", "indicators_1h", "indicators_4h", "symbols")
```

**Error:** Tool execution error: MCP error tableski: {"code":-32000,"message":"rejected: `information_schema.columns` is not a registered table (see list_tables)"}

## Q3: what type of information I can get from it?

```sql
SELECT
  SUM(close)
FROM bars_1d
WHERE
  symbol = 'ADA'
```

1 rows.

| sum(bars_1d.close) |
|---|
| 386.71271900000005 |
