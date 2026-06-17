# Coding assistant horde (planning tier)

DAG example for **1.4.0+**: operator form → parallel warmup + todo → join → fixed dev/test/review chain → `HANDOFF.md`.

**Scope today:** markdown planning artifacts in `output/` only — does **not** edit your project or run tests. See [`ROADMAP.md`](../../ROADMAP.md) § *Coding assistant horde (execution tier)* for tool stages, project tree ingest, and loops.

## Validate

```bash
cargo run -p kowalski-cli -- agent-app validate --path examples/coding-assistant
```

## Run (CLI, requires LLM at `http://127.0.0.1:3456`)

```bash
cargo run -p kowalski-cli -- agent-app run --path examples/coding-assistant \
  "Task: add structured logging. Project: /opt/ml/kowalski"
```

## UI / federation

1. Restart `kowalski` to reload the horde catalog.
2. **Federation** or **Horde** tab → **Coding assistant (planning)** → **Start All** workers.
3. Fill **Project path** + **Task specification** → **Run horde**.

Scheduling: `ingest` → (`warmup` ∥ `todo-plan`) → `adjust` → `dev-1` → … → `deliver` (parallel branches run sequentially in 1.4.0 MVP).

## Workers (manual)

```bash
cargo run -p kowalski-cli -- agent-app worker --role ingest --capability coding-assistant.ingest --path examples/coding-assistant coding-assistant-ingest
cargo run -p kowalski-cli -- agent-app worker --role process --capability coding-assistant.process --path examples/coding-assistant coding-assistant-warmup
# … one process worker per process step (same capability, unique agent id), plus deliver
```
