# TODO — 1.4.0 delivery (`feat/dag`)

> **Local checklist.** Sync with [`task_plan.md`](task_plan.md) phases. Check items here when done; log session work in [`progress.md`](progress.md).

**Current focus:** S5 release 1.4.0 — S4 UI DAG canvas complete

---

## S0 — Install & release hygiene

- [x] `install.sh` at repo root (crates.io install + config seed)
- [x] README one-line install section
- [x] CHANGELOG `[Unreleased]` install entry
- [ ] Merge `feat/dag` install commits (or cherry-pick) to `main`
- [ ] Verify: `curl -fsSL https://raw.githubusercontent.com/yarenty/kowalski/main/install.sh | bash` (after merge)
- [ ] Optional: `yarenty.com/kowalski/install.sh` → GitHub raw redirect
- [ ] Commit `scripts/publish-crates.sh` (fixed `--from`, no broken index wait)

---

## S1 — DAG schema & validation (`kowalski-core`)

- [x] Read `kowalski-core/AGENTS.md` + `markdown_pipeline.rs` + `rookery/*` before edits
- [x] Add `HordeEdge { from, to }` to manifest + `RookeryDraft`
- [x] Implement `resolve_execution_graph()` (implicit linear edges when `edges` absent)
- [x] DAG validation: acyclic, nodes ⊆ pipeline, duplicate edge policy
- [x] Update `validate_draft` + `validate_horde_tree`
- [x] Unit tests in `kowalski-core` (linear, fork/join, cycle, bad refs)
- [x] Document edge TOML shape in `kowalski-core/AGENTS.md` or `docs/` (Rule 7)

---

## S2 — Orchestrator scheduling

- [x] Refactor `agent_app_ops.rs` run loop: topological layers / ready set
- [x] HTTP horde run uses same scheduler (no duplicate logic in `kowalski/`)
- [x] Federation delegate/worker path still works for linear + DAG apps
- [x] Add [`examples/coding-assistant/`](examples/coding-assistant/) DAG planning horde
- [x] CLI validate smoke: `agent-app validate --path examples/coding-assistant`
- [ ] CLI + HTTP live run smoke (needs LLM + workers)

---

## S3 — Rookery

- [x] `draft_parse.rs` — coerce `edges` from LLM output
- [x] `writer.rs` — emit `[[edges]]` in born `horde.md`
- [x] `resources/prompts/rookery/builder.md` — fork/join guidance
- [x] MCP rookery tools — edges in validate/parse/birth payloads
- [x] Tests: birth a DAG draft to temp dir, `validate_horde_tree` passes

---

## S4 — UI

- [x] DAG layout in `PenguinCanvas.vue` (fork/join positions)
- [x] Rookery panel shows edges (read-only MVP OK)
- [x] Horde/Federation run UX mentions parallel branches
- [ ] Manual smoke: propose → birth → run DAG example in UI (operator)

---

## S5 — Ship 1.4.0

- [ ] Bump workspace to **1.4.0** (all crates + `ui/package.json`)
- [ ] `CHANGELOG.md` — promote `[Unreleased]` → `[1.4.0]`
- [ ] `ROADMAP.md` — mark DAG items shipped
- [ ] `cargo test --workspace`, `cargo clippy`, link check, CI
- [ ] Git tag `v1.4.0` + GitHub release
- [ ] `./scripts/publish-crates.sh` (all six crates)

---

## Backlog (not 1.4.0)

- [ ] A2A implementation (1.4/1.5) — see design doc
- [ ] Parallel in-process step execution (tokio)
- [ ] Prebuilt release binaries in install script
- [ ] Per-role penguin artwork (optional)
