# Coding assistant horde (planning tier)

DAG example for **1.5.0+**: operator form → parallel warmup + todo → join → fixed dev/test/review chain → `HANDOFF.md`.

**Scope today:** markdown planning artifacts in `output/` only — does **not** edit your project or run tests. See [`ROADMAP.md`](../../ROADMAP.md) § *Coding assistant horde (execution tier)* for tool stages, project tree ingest, and loops.

## Validate

```bash
cargo run -p kowalski-cli -- agent-app validate --path examples/coder
```

## Run (CLI, requires LLM at `http://127.0.0.1:3456`)

```bash
cargo run -p kowalski-cli -- agent-app run --path examples/coder \
  "Task: add structured logging. Project: /opt/ml/kowalski"
```

## UI / federation

1. Restart `kowalski` to reload the horde catalog.
2. **Federation** or **Horde** tab → **Coder (planning tier)** → **Start All** workers.
3. Fill **Project path** + **Task specification** → **Run horde**.

Scheduling: `ingest` → (`warmup` ∥ `todo-plan`) → `adjust` → `dev-1` → … → `deliver` (parallel branches run sequentially in 1.5.0 MVP).

## Workers (manual)

```bash
cargo run -p kowalski-cli -- agent-app worker --role ingest --capability coder.ingest --path examples/coder coder-ingest
cargo run -p kowalski-cli -- agent-app worker --role process --capability coder.process --path examples/coder coder-warmup
# … one process worker per process step (same capability, unique agent id), plus deliver
```
