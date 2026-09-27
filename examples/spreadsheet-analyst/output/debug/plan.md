### Q1: what is all about
The data primarily consists of budget calculator proposal basebox data and various market trading bar and indicator statistics.
```sql
SELECT
  SUM(col_1)
FROM "Sheet1"
```

### Q2: give me list of fields
The fields available are from the `Sheet1` table for budget calculation and from the market data tables like `bars_1d`, `bars_1h`, `bars_4h`, and `indicators_*`.
```sql
SELECT
  column
FROM information_schema.columns
WHERE
  table_name IN ("Sheet1", "bars_1d", "bars_1h", "bars_4h", "indicators_1d", "indicators_1h", "indicators_4h", "symbols")
```

### Q3: what type of information I can get from it?
You can retrieve cost and personnel data from the budget sheet or analyze historical market price movements, changes, and technical indicators for symbols like ADA and ZEC.
```sql
SELECT
  SUM(close)
FROM bars_1d
WHERE
  symbol = 'ADA'
```