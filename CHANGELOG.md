# Changelog

> "History is written by the victors, changelogs are written by developers who broke something." - Winston Churchill (probably)

All notable changes to this project will be documented in this file, or at least we'll try to remember to do so.

## [Unreleased]

## [1.7.0] - 2026-08-27 — **Autonomy**

> Consolidates the untagged in-tree milestones 1.5.0 (Coder execution tier) and 1.6.0
> (fresh install + mcp-base) with the durable-autonomy line: everything below is new
> since the 1.4.0 release.

### Changed

- **`kowalski-mcp-base` pre-extraction audit (#77):** the shared MCP server framework is
  being extracted into its own project, **emperor-mcp** (#75). In-tree preparation:
  `#![warn(missing_docs)]` with every public item documented, unused `log` dependency
  dropped (single `tracing` facade), MSRV declared (`rust-version = "1.85"`), neutral
  standalone crate description, and a README introducing the emperor-mcp identity.
  No API or behavior changes; `kowalski-mcp-rookery` / `kowalski-mcp-datafusion` unaffected.

### Security

- **Optional HTTP API auth (#36):** the server can require a locally generated bearer
  token on `/api/*` (`Authorization: Bearer <token>`, or `?token=` for SSE/WebSocket).
  **Off by default** — this is a single-user local tool; enable with the `--auth` flag,
  `[server] auth = true` in `config.toml`, or by setting `KOWALSKI_API_TOKEN`. When
  enabled: the token is generated on first server start, printed once, and persisted with
  mode 0600 at `<config-dir>/db/api_token`; `/api/health` stays open; permissive CORS is
  replaced with an origin allowlist (Vite dev UI by default; configure via `--cors-origin`
  / `[server] cors_origins`). The UI sends the token from a Home-tab settings field
  (localStorage, `VITE_API_TOKEN` fallback); CLI workers use the `KOWALSKI_API_TOKEN` env
  var, set automatically for server-spawned workers.

### Added

- **Export/import surfaces (#70):** portable bundles are now one command / one click
  away on every surface. CLI: `kowalski-cli agent-app export <horde> [-o out] [--json]`
  (horde directory path or an id resolved across the horde discovery roots) and
  `agent-app import <bundle> [--dir <hordes-root>] [--json]`, both printing the
  portability report (human or JSON); import rejects invalid bundles with a non-zero
  exit. HTTP: `GET /api/hordes/{id}/export` streams the `<id>-<version>.kwf.zip`
  bundle and `POST /api/hordes/import` (multipart upload, `?dry_run=true` for a
  report-only pass) lands the draft and returns the report + created horde id —
  auth applies like every `/api/*` route, invalid bundles come back as 400 with the
  validation message. UI: an **Export** button on each horde card downloads the
  well-named bundle and an **Import bundle…** action on the hordes view uploads,
  shows the portability report, and lands the horde on confirm — it appears without
  a restart via the hot-reload catalog. Imports always land in the **user hordes
  root** (first `KOWALSKI_HORDES_DIR` entry, else `<config-dir>/hordes`), never in
  `examples/`; the server and CLI activate the live portability checks by passing
  their registered tool ids and reachable Ollama models. Shared owners moved to
  `kowalski_core::config`: `default_horde_roots`, `user_hordes_root`,
  `ollama_base_url`, `HORDES_DIR_ENV`; `kowalski_core::manifest` gains
  `inspect_bundle` (every import gate, nothing written) backing the dry-run flow.
- **Portable workflow bundle (#69):** workflows now travel as a single file —
  `<identifier>-<version>.kwf.zip`, containing `manifest.json` (byte-identical to the
  stored manifest) plus optional `assets/` files; import also accepts the `.bbwf.zip`
  extension (identical format, an interop alias for externally produced workflow
  bundles). `kowalski_core::manifest` gains `export_horde_dir_bundle`/`export_bundle`
  (Publish-validated; bundles never contain credentials, run history/state,
  watched-directory contents, or server-side trigger-override state — audited by test)
  and `import_bundle`, which treats the zip as untrusted: entry allowlist, zip-slip
  guard, symlink rejection, and size caps, then `schema_version` gating (different
  MAJOR rejected, newer MINOR rejected with an upgrade hint, older MINOR migrated
  through registered migration scaffolding), semantic validation, and landing as a
  **draft** horde directory that is never auto-armed — every imported trigger
  declaration is rewritten to `enabled = false` so the operator re-enables
  deliberately. Import returns a **portability report** (unknown step kinds, unknown
  tool providers, locally unavailable builtin tool ids, unresolved pinned models —
  checked against a caller-supplied `PortabilityContext`; reported, never fatal), and
  the full export→import round-trip is equivalence-tested on `examples/coder`.
- **Portable workflow manifest (#68):** hordes now have a canonical **JSON interchange
  form** next to the markdown authoring format. `kowalski_core::manifest` ships a strict
  `WorkflowManifest` model (`deny_unknown_fields`, `schema_version` 1.0, kebab
  identifier, semver version, steps with inlined prompts / tool bindings / `ui` operator
  form, `pipeline[]` + `edges[]` 1:1 with the horde DAG declaration, and `kowalski`
  extension blocks carrying triggers — every field incl. `overlap` — plus delivery and
  step runtime config), a published JSON Schema asset
  (`kowalski-core/resources/schemas/workflow-manifest.schema.json`, draft 2020-12), and
  two-tier semantic validation (`Draft` vs `Publish` strictness; unknown step kinds are
  portability warnings, not errors). Converters go both ways: `horde_dir_to_manifest`
  exports any horde directory (prompt files inlined, writer defaults resolved so the
  manifest is self-contained), and `write_manifest_tree` validates then regenerates the
  markdown tree through the rookery writer. Round-trip tested on `examples/coder` and
  `examples/knowledge-compiler` (dir → manifest → dir preserves semantics; manifest →
  dir → manifest is byte-stable; DAG edges, conditional retry loops, and triggers survive
  exactly). Manifests never contain credentials, run history, document contents, or
  server-local trigger overrides. Supporting fixes: the rookery writer now emits
  `tool_ids`, `model_id`, `verify_command`/`verify_cwd`, `apply_mode`, `isolation`, and
  `normalize_*` agent fields (previously dropped on birth), skips prompt files for
  prompt-less deterministic steps, orders `[[inputs]]` blocks last so following keys are
  not swallowed into the inputs table, and honors per-step `capability` /
  `default_agent_id` overrides; horde.md accepts an optional `version`; born-horde
  validation no longer demands prompts on deterministic (`verify`/`apply`/`ingest`)
  steps.
- **Trigger operator UI (#64):** triggers are now visible and controllable from the
  operator UI. The **Horde** tab gains a **Triggers** panel per horde: each trigger shows
  a kind badge with its schedule/route/path, armed/disabled state, next cron fire, and a
  link to the run it last fired; an **Enable/Disable** toggle records a server-side
  operator override (persisted at `<config-dir>/db/trigger_overrides.json` — never by
  editing `horde.md` — and it survives restarts: overrides un-arm cron/watch triggers,
  release webhook routes, and are re-checked before queued firings), and **Fire now**
  starts the trigger's run immediately for testing (works while disabled; the overlap
  policy still applies). A new **Recent runs** feed badges every run's source
  (cron/watch/webhook vs operator) and marks interrupted-then-resumed runs with
  `resumed ×N`; Federation horde cards show the same trigger badges. New thin API
  endpoints over the trigger runtime: `GET /api/hordes/{id}/triggers` (status incl.
  last-fired from the run store) and `POST /api/hordes/{id}/triggers/{index}/enable`,
  `…/disable`, `…/fire`; the `triggers` array on `GET /api/hordes` (+ detail) is enriched
  with `index`/`kind`/`detail`/`effective_enabled`/`overridden` so the UI's 15 s listing
  poll reflects toggles. Trigger definitions stay authored in `horde.md` / Rookery — the
  UI does not edit them.
- **Trigger runtime — event-driven horde runs (#63):** the `[[triggers]]` declared in
  `horde.md` (#62) now fire. The server arms every enabled trigger from the horde catalog
  at startup and re-arms on hot reload: **cron** schedules run on a dependency-free minute
  scheduler (5-field expressions, local time — the runtime consumes the #62 parser, no
  cron crate); **watch** triggers share one filesystem-watcher instance with per-trigger
  debounce, and the deduplicated changed paths become the run input; **webhook** triggers
  are served at `POST /api/triggers/{route}` (bearer auth like any `/api/*` route when
  enabled; 404 for unknown or disabled routes, 409 for cross-horde route collisions) with
  the JSON body as `{{trigger.payload}}`. A firing is an ordinary durable run —
  `origin = "trigger"`, so a server restart **auto-resumes** it, with
  `source = "trigger:<kind>:<horde>"` and a `trigger_fired` run event (changed paths /
  webhook payload included) for the audit trail; failures to fire are logged loudly and
  never crash the server. New per-trigger `overlap` field (also validated, editable in
  Rookery, and emitted at birth): while a previous run from the same trigger is in flight,
  `"skip"` (default) drops the firing and records `trigger_skipped` on the in-flight run,
  `"queue"` parks at most one firing (newer ones coalesce) and fires when the run ends,
  `"parallel"` always starts a run. Runs paused awaiting operator input don't count as in
  flight, so a parked run never wedges its trigger.
- **Triggers in the horde manifest (#62):** hordes can now declare event-driven runs in
  `horde.md` frontmatter via a `[[triggers]]` TOML array — three kinds: `cron = "0 7 * * *"`
  (5-field, local time; dependency-free parser), `watch = { path, events, debounce_ms }`
  (events ⊆ create/modify/remove, default `["create","modify"]` / 2000 ms), and
  `webhook = { route }` (kebab-case slug, unique across all hordes; reserved under
  `/api/triggers/`). Common optional fields per trigger: `enabled` (default true), `input`
  (pre-filled operator-form answers by field id), and `prompt` (run prompt template with
  `{{trigger.path}}` / `{{trigger.payload}}` / `{{trigger.time}}` placeholders). This release
  is the **declaration layer**: triggers are parsed and validated (precise errors for bad
  cron, duplicate routes, unknown keys; missing watch paths are a warning), carried on the
  run's manifest snapshot, listed read-only in `GET /api/hordes`, reported by
  `agent-app validate`, and editable in Rookery (draft field + `set_triggers` delta op +
  written to `horde.md` at birth). The firing itself lives in the trigger runtime (#63,
  above).
- **Rookery guided interview — per-turn deltas (#58):** the horde builder no longer asks the
  model to emit one complete draft document. Each chat turn now runs two channels: an **ops
  phase** that asks the model for a small batch of typed edit operations (constrained to the
  delta schema when structured output is enabled; fenced-JSON extraction otherwise — and
  un-parseable output degrades to a clean "no edits" turn) which the server applies to the
  session draft with prefix-apply (a rejected op keeps the last-good draft and is explained
  to the operator), and a **reply phase** that answers in plain prose, informed by a note
  about what was actually applied. The draft pane updates after every turn; the draft id is
  server-owned (derived from the display name until birth); `replace_draft` (whole-document
  emission) stays off unless explicitly enabled. Ops are recovered by JSON extraction by
  default — empirically the most reliable path on Ollama, where grammar-constrained decoding
  over the multi-op schema can distort op choice — with `[llm] structured_output = true`
  switching to constrained emission on backends where guided decoding is known-good. New
  optional `[rookery]` config:
  `max_ops_per_turn` (default 12), `structured_output` (default true; effective only with
  `[llm] structured_output`), `allow_replace_draft` (default false). The builder system
  prompt was rewritten for op-at-a-time building; the whole-draft propose endpoint,
  give-birth flow, and the MCP rookery server are unchanged. Also fixed: born hordes now
  keep `when` / `max_loops` on `[[edges]]` (the writer previously dropped them).

- **Constrained structured output on LLM providers (#57):** opt-in `[llm]
  structured_output = true` adds `chat_with_schema(model, messages, schema)` to the
  provider layer — the model's reply is *forced* to conform to a JSON Schema and comes
  back as raw JSON text. Ollama carries the schema in the request's `format` field;
  OpenAI-compatible servers get strict `response_format: json_schema` (this is also how
  guided-decoding servers expose the feature). `supports_structured_output(model)` reports
  the capability and the default implementation degrades gracefully, so callers fall back
  to prompt-based extraction — the new `utils::json::extract_first_json_object` recovers a
  JSON object from plain, fenced, prose-embedded, or repairably-truncated replies. The
  conservative schema-subset guard now runs at request-build time on both providers, and
  additionally rejects length/count bounds over 1024 (grammar-based backends compile them
  into bounded repetition and reject the request; large prose caps were dropped from the
  builder delta schema accordingly). Verified live against a 3B local model emitting a
  valid builder delta batch under the schema.

- **Rookery builder delta operations (#56):** the horde builder gains a delta layer in
  `kowalski-core` — instead of emitting one complete draft document (which breaks on small
  local models), a builder LLM can emit small, typed edit operations that the server applies
  to the draft. `DeltaOp` covers `set_meta`, `add_step`, `update_step`, `set_prompt`,
  `bind_tool`, `unbind_tool`, `remove_step`, `reorder`, `set_edges` (edges are first-class in
  kowalski hordes), and `replace_draft` (whole-document escape hatch for capable models);
  `apply_batch` applies a `DeltaBatch` in order with prefix-apply semantics — validate after
  each op, roll back to the last good state on the first failure, and report
  `{ applied, error: { index, message } }` so the failure can be fed back to the model. Draft
  validation gains a lenient `DraftStrictness::Draft` mode for in-progress drafts (structural
  rules enforced; completeness deferred to birth). The batch contract ships as a JSON Schema
  asset (`kowalski-core/resources/schemas/rookery-delta.schema.json`) kept inside a
  conservative subset that constrained-decoding backends support, with a schema-variant
  builder (per-turn ops cap, `replace_draft` stripped unless opted in) and a mechanical
  subset guard. The draft `id` stays server-owned — no op can edit it. LLM structured-output
  plumbing and the interview flow land in follow-up work.

- **Horde catalog hot reload (#52):** horde definitions are no longer frozen at server
  startup. The new `HordeCatalog` re-resolves a horde from disk when its definition files
  change and re-scans the horde roots on every listing — **adding, editing, or removing a
  horde directory takes effect without a restart** (a single debounced file watcher also
  pushes refreshes and hooks new run topics into the orchestrator). In-flight runs are
  immune: the orchestrator advances, resumes, and cancels runs from the run's persisted
  manifest snapshot, so a definition edited mid-run applies from the next run only. A
  broken edit never drops a horde — the server keeps the last good version and surfaces
  the parse error on `/api/hordes` (`load_error`); the UI Horde tab polls the listing,
  shows a ⚠ badge with the error, and clears it once the file is fixed. Hot-added hordes
  get their workdir prepared on first reference; reloads never re-run `clean_on_startup`.

- **Agent loop: native tool calls first, ReAct as fallback (#51):** every
  `chat_with_tools*` entry point — REPL chat, `/api/chat` (plain and streaming), in-process
  horde LLM steps, isolated `exec-step` children, and role workers — now runs the
  provider's **native structured tool loop** when the model supports it: tool declarations
  travel on the wire, calls execute in order (multiple per turn), and results feed back as
  proper `role = "tool"` messages instead of being spliced into prompt text. `[llm]
  tool_calling = "auto" | "native" | "react"` (default `auto`) overrides the choice per
  deployment: `auto` follows the `native_tools` capability flag, `native`/`react` force one
  path. The ReAct text loop is unchanged and remains the fallback; both loops share the
  same iteration cap, and the native loop gets its own runaway guard (identical consecutive
  tool calls short-circuit). Tool schemas for the prompt appendix and the wire now come
  from one place (`ToolDefinition::from_tool`).

- **Native tool calling on the LLM provider layer (#50):** `LLMProvider` gains
  `chat_with_tool_defs(model, messages, tools) -> ChatOutcome` — tool declarations travel
  on the wire and tool-capable models answer with **structured tool calls** instead of
  JSON embedded in free text — plus `supports_native_tools(model)`. Opt in with
  `[llm] native_tools = true` in `config.toml` (default off; providers without the opt-in
  behave exactly as before, and the text-based ReAct loop remains the fallback). Both
  backends are wired: Ollama (`tools` in `/api/chat`, `message.tool_calls` parsed, ids
  synthesized) and OpenAI-compatible servers (typed Chat Completions tools). Tool results
  return as `role = "tool"` messages (`Message::tool_result`); `ToolDefinition::from_tool`
  converts `Tool` metadata to the wire JSON Schema in one place, and conversation
  persistence round-trips the new `tool_calls` / `tool_call_id` message fields (old
  conversations still load).

- **Opt-in process isolation for horde steps (#42):** a step's `agents/*.md` frontmatter
  can declare `isolation = "process"` (default `in_process`, validated at load, captured
  in the run's manifest snapshot) to execute that step in a spawned one-shot child
  process instead of the server — for hordes you don't fully trust. The orchestrator owns
  the child per step (spawn → execute → reap, pid logged as evidence): it writes an
  `IsolatedStepRequest` JSON document to the child's stdin and consumes
  `message`/`outcome` JSON lines from its stdout; run cancellation and the per-step
  timeout kill the child (no orphans). The child is the new `kowalski-cli agent-app
  exec-step` command, a thin client of the same `StepHandlerRegistry` the server runs
  in-process (`kowalski_core::execute_isolated_request`), so artifacts and pass/fail
  routing are identical; LLM kinds talk to the configured provider directly — an isolated
  child never calls the server API. Child binary resolution: `KOWALSKI_CLI_BIN` env >
  sibling `kowalski-cli` next to the server binary > `cargo run -p kowalski-cli` (dev
  tree). The SSE federation worker (`agent-app worker --role …`) was refit onto the same
  shared execution path: its per-role handlers (and the `/api/chat` LLM loopback) are
  gone. Model resolution now has a single owner: `kowalski_core::config::default_model`.
- **In-process LLM steps, per-step timeouts, run cancellation (#41):** horde runs no
  longer spawn any worker processes by default. LLM step kinds (`process`/`step`/
  `deliver`/`final`, `compile`, `ask`, `lint`) execute inside the server via
  `LlmStepHandler` — prompt assembly plus a direct call into a dedicated horde
  `TemplateAgent` (tool allowlists and sandbox policy preserved) instead of the old
  CLI-worker HTTP self-loopback through `/api/chat`. Every in-process step runs under a
  wall-clock timeout (`[horde] step_timeout_secs`, default 600); a timed-out or failed
  step now routes over a matching `fail` edge when the DAG has one (loop retries), and
  only fails the run otherwise. New `POST /api/hordes/{id}/runs/{run_id}/cancel`:
  cooperative cancellation stops the in-flight step, skips the remaining steps, and
  persists the run as `cancelled` (new `run_cancelled` event + `RunCancelled` federation
  message); the UI gains a **Cancel run** button. Startup now skips a horde's
  `clean_on_startup` workdir clean while it has interrupted runs pending resume, so
  resumed runs keep their completed artifacts. Worker execution stays in the CLI as the
  future opt-in isolation mode.
- **In-process step execution — `StepHandler` trait + registry (#40):** deterministic
  horde step kinds (`verify`, `apply`, `ingest`) now execute inside the server process
  instead of spawned federation workers: same artifacts, same pass/fail routing on
  conditional edges, no worker process (they disappear from worker profiles / Start All).
  LLM kinds keep the worker path in the same run (mixed mode); a horde whose steps are
  all deterministic runs with zero workers. New `kowalski_core::horde_step` module —
  adding a step kind is one `StepHandler` impl plus one registry line (see
  `kowalski-core/AGENTS.md`). Agent frontmatter `verify_command` / `verify_cwd` /
  `apply_mode` are now part of the server-side horde spec and the run's manifest
  snapshot.
- **Fixed:** artifact chaining for steps that are also loop-back retry targets (e.g.
  `verify --fail--> dev`): the previous-artifact lookup now ignores loop-back edges
  (`single_forward_predecessor`), so such steps receive their real upstream artifact —
  previously they got none and LLM stages using `@artifact@` failed mid-DAG. Fixed in
  both the server orchestrator and the local `agent-app run` runner.
- **Resume interrupted horde runs (#39):** your agents survive a reboot. On startup the
  server scans the run store for incomplete runs: interrupted runs are surfaced as
  resumable (`GET /api/hordes/{id}/runs?status=resumable`, `resumable` flag on run
  records) and runs started with a non-`operator` `origin` (new optional field on
  `POST /api/hordes/{id}/run`, built for future trigger-fired runs) are auto-resumed.
  `POST /api/hordes/{id}/runs/{run_id}/resume` resumes a run on demand: the step that was
  in flight at the kill is retried as a new attempt, the next ready step is recomputed
  from the manifest snapshot graph plus persisted step outcomes (conditional-loop counts
  intact — no extra iterations), and completed steps' artifacts are never regenerated.
  `awaiting_input` runs stay parked and resume on demand. Resume attempts are capped
  (`[horde] resume_max_attempts`, default 2); an exhausted run goes to `error` with a
  clear reason. The UI Horde tab shows an "Interrupted runs" banner with a Resume button,
  and resumed runs carry a marker in the event feed. Migrations
  `sqlite/004_run_resume.sql` + `postgres/006_run_resume.sql` (adds `origin`,
  `resume_count`).
- **Persisted horde-run store (#37):** new `kowalski_core::db::run_store` — typed
  `RunStatus`/`StepStatus` state machine, `manifest_snapshot` at run start, per-step
  `attempt`/`outcome`/artifact, compact JSON events log, and `incomplete_runs()` for restart
  scans. Zero config: creates `runs.sqlite` under the server state dir (`KOWALSKI_RUN_DB`
  overrides). Migrations `sqlite/003_horde_runs.sql` + `postgres/005_horde_runs.sql`.
- **Orchestrator write-through (#38):** the horde orchestrator now persists every run and
  step transition to the run store — run created (with a manifest snapshot of the loaded
  horde spec), step delegating/succeeded/failed, loop counts and per-step attempts, run
  done/error, plus the full event feed — so completed runs survive a server restart and a
  run interrupted mid-flight stays visible (status `running`, completed steps recorded).
  `RunRecord`/`RunStepRecord` statuses are now typed enums; `/api` responses keep the
  historical wire vocabulary (`completed`/`failed`/`success`). Run listing
  (`GET /api/hordes/{id}/runs`) reads from the store and is paged (`?limit=` up to 500,
  `?offset=`); run detail and follow-up chat also work for runs from before a restart.
  The in-memory registry remains as the active-run cache.


### Fixed

- **Stage `output` directories:** a trailing-slash `output` (e.g. `debug/summaries/`) crashed
  LLM stages with `No such file or directory`; the stage runner now writes `<dir>/<step>.md`
  into it (fixes `examples/url-summarizer`).
- **Workers follow `--bind`:** the server exports its real base URL to spawned workers via the
  new `KOWALSKI_API` env var instead of workers assuming `127.0.0.1:3456`. The CLI resolves its
  target as `--api` flag > `KOWALSKI_API` > default. Shared constants now live in one place:
  `kowalski_core::config::{DEFAULT_API_BIND, API_URL_ENV, API_TOKEN_ENV}` (see new `AGENTS.md`
  Rule 8: single source of truth).

> Workspace **`1.5.0`** on `feat/coder`: **Coder execution tier** — project tree ingest, tool-enabled federation stages, verify/apply, conditional loops. See [`ROADMAP.md`](ROADMAP.md) § *Coder execution tier*.

### Added (in progress)

- **OpenAI-compatible provider model selection:** Added `llm.model` field to `config.toml`. When `llm.provider = "openai"`, the system now uses `llm.model` as the primary model name; falls back to `ollama.model` if `llm.model` is not set. This allows different models for Ollama vs OpenAI-compatible APIs.
- **HTTP API model determination:** `/api/chat` and `/api/chat/stream` now use `determine_model()` to select the correct model based on provider config.
- **CLI model determination:** `kowalski-cli chat` and `kowalski-cli run` use the same `determine_model()` logic.
- **UI accurate provider/model display:** `/api/doctor` endpoint now shows the correct model based on provider config.

### Added (in progress)

- **Project tree ingest:** `source_bundle` walks local `project_path` from operator form (ignore `.git`, `target`, `node_modules`, caps on file count/bytes); intake artifact includes manifest + selected file contents for warmup stages.
- **Operator `path` field type:** server validates that answered paths exist and are directories.
- **Tool-enabled horde stages:** `tool_ids` on `agents/*.md`; federation worker calls `POST /api/chat` with `use_tools`, allowlist, and `sandbox_root` from operator `project_path`. Built-in **`fs_tool`** (`list_dir`, `read_file`, `write_file`, …) registered on the HTTP agent.
- **Verify / apply stages:** `kind = "verify"` runs `verify_command` in `project_path` and writes a artifact with `status: pass|fail`. `kind = "apply"` dry-runs ```diff blocks; execute gated by `KOWALSKI_HORDE_APPLY=1`.
- **Conditional edges:** `[[edges]]` support `when = "pass"|"fail"` and loop-back edges with `max_loops`. Orchestrator and local `agent-app run` route on verify outcome, reset the retry span, and cap loops. Coder example: `test-verify` fail → `dev-1` (max 2), pass → `review`.
- **MCP framework:** Renamed **`kowalski-mcp-transport` → `kowalski-mcp-base`**; added output framing, credential forwarding, and rmcp `serve` bootstrap. Authoring rules: [`kowalski-mcp-base/MCP_REQUIREMENTS.md`](kowalski-mcp-base/MCP_REQUIREMENTS.md), manifests: [`kowalski-mcp-base/MANIFEST_SPEC.md`](kowalski-mcp-base/MANIFEST_SPEC.md). MCP server crates are optional workspace members (`default-members` = core only). Added `manifest.yaml` to `kowalski-mcp-datafusion` and `kowalski-mcp-rookery`. Removed staging `mcp-base/`.

## [1.4.0] - 2026-06-15 — **DAG pipelines + planning Coder**

> Published on crates.io as **`1.4.0`**: **DAG horde pipelines** (`edges[]`), graph orchestrator scheduling, Rookery DAG birth + UI canvas, **`install.sh`**, coding horde example (planning tier), federation worker fixes for custom stage kinds.

### Added

- **One-line install:** [`install.sh`](install.sh) — `curl -fsSL https://raw.githubusercontent.com/yarenty/kowalski/main/install.sh | bash` installs `kowalski-cli` and `kowalski` from crates.io, seeds `~/.config/kowalski/config.toml`, and documents optional MCP / postgres feature flags.
- **DAG horde graph:** optional `[[edges]]` on `horde.md` / `RookeryDraft`; `kowalski_core::horde_graph::resolve_execution_graph()` validates acyclic graphs, pipeline topological order, and returns scheduling layers. Empty/missing `edges` → implicit linear chain (existing hordes unchanged).
- **DAG orchestrator scheduling:** `agent-app run` and the HTTP horde orchestrator execute steps via `execution_order()` / `next_ready_step()` (sequential within each ready layer in MVP). Federation workers resolve `@step:name@` via on-disk outputs.
- **Rookery DAG birth:** `write_horde_tree` emits `[[edges]]` when the draft graph differs from an implicit linear chain; builder prompt documents fork/join. MCP rookery tools accept `edges` in draft JSON.
- **UI DAG canvas:** **PenguinCanvas** layered fork/join layout; Rookery read-only edge list; Horde/Federation DAG scheduling notes.
- **Coding horde example** (planning tier; rebranded to [`examples/coder/`](examples/coder/) on `feat/coder`): operator form (project path + task) → parallel warmup + todo-plan → adjust → dev/test/review chain → handoff markdown (repo edits deferred to **1.5.0**).

### Fixed

- **Federation workers for custom stage kinds:** `agent-app worker` now runs LLM stages for `process`, `step`, `deliver`, and `final`.
- **Ingest:** directory paths in operator input no longer treated as single files (`source_bundle` skips non-file paths).

## [1.3.0] - 2026-06-14

> Workspace and crates **`1.3.0`**: **Rookery** horde builder (HTTP + UI + MCP), server-owned sessions, in-repo MCP transport unification, Docker MCP gateway, penguin avatars, and operator-form validation on the server.

### Added

- **A2A federation-edge design:** [`docs/DESIGN_A2A_FEDERATION_EDGE.md`](docs/DESIGN_A2A_FEDERATION_EDGE.md) — decision + mapping for adopting [A2A](https://a2a-protocol.org/) **only** at the node↔node boundary (Agent Card derived from `AgentRegistry` + horde catalog; A2A Task lifecycle mapped onto existing `AclMessage` variants; transport reused from `kowalski-mcp-base`). Explicitly **no penguin-to-penguin A2A**; implementation deferred to 1.4/1.5.
- **Stateless Streamable HTTP for in-repo MCP servers:** new shared crate **`kowalski-mcp-base`** provides one `McpHandler` trait and two runners — **stdio** and **stateless Streamable HTTP** (no `Mcp-Session-Id` issued or required; every POST independent → restartable / horizontally scalable). Both **`kowalski-mcp-rookery`** (`--transport stdio|http`, `--bind`) and **`kowalski-mcp-datafusion`** now run on it, so every in-repo MCP server is reachable over stateless HTTP. The Kowalski MCP client already tolerates sessionless servers (captures `Mcp-Session-Id` only if present). See [`kowalski-mcp-base/README.md`](kowalski-mcp-base/README.md).
- **Rookery MCP server:** new in-repo crate **`kowalski-mcp-rookery`** — an MCP server (stdio **or** stateless HTTP) that exposes the horde builder so any MCP client (the Kowalski agent, CLI, or external clients like Claude Desktop) can build hordes, not only the Vue tab. Tools: **`rookery_example_draft`**, **`rookery_validate_draft`**, **`rookery_parse_draft`**, **`rookery_give_birth`** — all delegate to `kowalski_core::rookery` (same primitives as `/api/rookery/*`, no duplicated orchestration). The server is **LLM-free**: the calling agent drives the interview; this server validates/parses/writes. Wire it via `config.toml` and verify with `kowalski-cli mcp ping`/`mcp tools`. See [`kowalski-mcp-rookery/README.md`](kowalski-mcp-rookery/README.md).
- **Docker MCP gateway support:** Kowalski connects to the [Docker MCP Toolkit](https://docs.docker.com/ai/mcp-catalog-and-toolkit/toolkit/) catalog through **one** stdio MCP server (`command = ["docker", "mcp", "gateway", "run"]`) instead of wiring many individual servers — verified via `kowalski-cli mcp ping`/`mcp tools`. Default (no flags) exposes the gateway's **dynamic** management tools (`mcp-find`, `mcp-exec`, `code-mode`); `--servers <name>` / `--profile <id>` expose a specific server's tools by name (after it is configured in Docker Desktop). `tools/internal/*` remain the dependency-light fallback and are shadowed by the gateway when present. Documented in [`config.toml`](config.toml) and [`kowalski-core/AGENTS.md`](kowalski-core/AGENTS.md).
- **Rookery horde builder:** `kowalski-core::rookery` — linear draft validation and `write_horde_tree` for born hordes; builder prompt at [`resources/prompts/rookery/builder.md`](resources/prompts/rookery/builder.md). HTTP API on the `kowalski` server: `POST/GET/DELETE /api/rookery/sessions`, `POST .../chat` (optional SSE via `"stream": true`), `POST .../propose`, `POST .../give-birth`. Vue **Rookery** tab: interview chat, pipeline summary, **Give birth**. Default output root: `examples/` (override with `KOWALSKI_ROOKERY_OUTPUT` or `give-birth.output_root`).
- **Penguin avatars in UI:** per-step mascot images from [`ui/src/assets/pinguins/`](ui/src/assets/pinguins/) — auto-assigned on **Propose** from `kind` + step `name` (`kowalski-core::rookery::infer_penguin_avatar`), persisted in born horde `agents/*.md` frontmatter as `avatar = "…"`, editable per penguin in **PenguinEditor** (avatar picker). Shown on **PenguinCanvas**, Rookery/Chat/Federation run feeds.

### Changed

- **Horde operator forms are server-validated (thin UI / thick core):** `POST /api/hordes/{id}/run` now accepts a structured `form_answers` map; the **server** validates it against the horde's `run_form` (required / `url` / `choice` rules via `kowalski_core::validate_form_answers`, 400 on error) and builds the operator-input prompt block via `kowalski_core::answers_to_prompt`. The Vue **Horde Run** form no longer hand-assembles that block or enforces field rules client-side — it sends the raw answers.
- **UI:** the duplicated SSE line-pump in `ui/src/api.ts` is extracted into one `streamSse<T>()` helper shared by `chatStream` and `rookeryChatStream`.
- **`kowalski-mcp-datafusion` is now stateless:** it no longer generates or echoes an `Mcp-Session-Id` (the per-process `uuid` session was removed), and its HTTP/SSE plumbing moved to `kowalski-mcp-base`. `AppState::new(ctx, table)` drops the former `session_id` argument. Tools and wire shapes are unchanged.
- **Rookery — server-owned draft:** the `kowalski` server now **persists each Rookery session** (status, draft, summary, chat transcript) as one **YAML** file under `db/rookery/` (override `KOWALSKI_ROOKERY_STATE`) and **reloads them on startup**, so sessions survive a restart without the browser re-POSTing the draft. New `GET /api/rookery/sessions` lists server-owned sessions. The Vue **Rookery** tab now keeps only a thin session-id list in `localStorage` and hydrates draft/status via `GET /api/rookery/sessions/{id}`; the legacy `POST` restore body is still accepted but no longer used by `ui/`.
- **Rookery:** per-penguin editor (`PenguinEditor.vue`), `PATCH /api/rookery/sessions/{id}/penguins/{name}`, `POST .../save-horde` to flush draft edits to disk after give birth; session recovery on server restart.
- **Rookery:** `normalize_draft` slugifies LLM-produced horde/penguin ids (e.g. `Ingest` → `ingest`, `rust_project_scaffolder_1.0` → `rust-project-scaffolder-1-0`) before validation; builder prompt documents kebab-case id rules. `parse_draft_from_assistant` coerces common LLM JSON mistakes (objects instead of strings for `description`/`output`, object entries in `pipeline`).

## [1.2.0] - 2026-05-03

> Workspace and crates **`1.2.0`**: Knowledge Compiler layout simplification (`debug/raw`, `debug/reports`, `debug/lint`, `debug/followups`, `agents_log`), horde HTTP artifact conventions, GitHub-aware ingest, and operator docs/UI updates (full list below).

### Added

- **Knowledge Compiler:** [`examples/knowledge-compiler/AGENTS.md`](examples/knowledge-compiler/AGENTS.md) operator guide (`GITHUB_TOKEN`, MCP vs CLI).
- **GitHub-aware URL fetch** as an **internal tool** in `kowalski-core`: [`tools/internal/github.rs`](kowalski-core/src/tools/internal/github.rs) (README API + `raw.githubusercontent.com`, optional `GITHUB_TOKEN`, plain-HTTP fallback). **HTML → readable Markdown** heuristics in [`tools/internal/web.rs`](kowalski-core/src/tools/internal/web.rs); ingest bundling moved to [`source_bundle.rs`](kowalski-core/src/source_bundle.rs) so **`kowalski-cli` stays an executor**. **`KOWALSKI_AGENT_APP_ROOT`** overrides the dev-only default app path. See **Tool sources** / **Strict boundaries** in AGENTS files.
- **Obsidian operator flow (paste-first):** each run ends with **`workdir/PASTE_ME.md`** (and **`run_finished.paste_for_obsidian`** in the UI) — one markdown block for copy-paste. All intermediate pipeline output (ingest, wiki, reports, scratch) is under **`workdir/debug/`**. **Horde Run** panel: **Copy to clipboard**.
- **Concept wikilink repair** extends to concept→concept links with reciprocal **Related Concepts** backlinks.
- **Operator UI:** root + `ui/` + `kowalski-core` **AGENTS** now state UI-first smoke acceptance (Horde / Federation / Chat); Federation panel help text updated for `agent-app` + Horde tab (removed stale `extension run`).
- **`ui/README.md`:** **Operator smoke checklist (~2 minutes)** (Home, Chat, Federation **Start All**, Horde **Run Horde**, optional registry refresh).
- **Horde workdir:** `POST /api/hordes/{horde_id}/clean-workdir` runs the same filesystem cleanup as **clean on startup** (removes `debug/`, legacy top-level `raw/` / `wiki/` / `scratch/`, `agents_log/`, and workdir `PASTE_ME.md`). Vue **Horde Run** and **Federation → Horde cards** include an inline **Clean now** control next to “Clean on startup”. After a successful clean, the UI starts a **new Chat** session (`POST /api/chat/reset`), matching sidebar **New conversation**.

### Fixed

- **`markdown_pipeline` normalization:** for non-empty LLM output, optional `normalize_sections` no longer appends extra `##` headings using substring checks — models that use emoji or alternate titles (e.g. `## 📝 TL;DR` instead of `## TL;DR`) were falsely treated as “missing” sections, which duplicated headings and injected `normalize_fallback` text (e.g. “Model output was empty…”) into otherwise valid handoff files. Synthesis from **truly** empty output is unchanged.
- **HTML → markdown (internal web ingest):** `<a href="…">` is converted to **`[text](url)`** before tag stripping so hyperlinks survive the heuristic HTML pass (plain tag removal previously dropped URLs entirely).
- **`agent-app worker`** (federation SSE): the blocking HTTP client used the default **30s** request timeout, so idle **`GET /api/federation/stream`** reads failed and stderr showed `federation stream decode warning (ignored): error decoding response body` (reqwest’s misleading label for that timeout). The stream client now disables that timeout for the long-lived connection.

### Changed

- **Knowledge Compiler:** ingest bundles live under **`workdir/debug/raw/`** (was **`debug/raw/sources/`**); **`kowalski_core::source_bundle`**, `agent-app`, and example docs updated.
- **Knowledge Compiler:** removed the **`debug/derived/**` tree — ask output is **`workdir/debug/reports/`**, Marp/slides prompts use **`debug/slides/`**; `agent-app`, **`agents/ask.md`**, prompts, and README updated. Horde startup cleanup no longer references top-level **`derived/`** or legacy **`derived/obsidian-paste.md`**.
- **Knowledge Compiler:** lint report path is **`workdir/debug/lint/`** (was **`debug/derived/lint/`**); `agent-app`, **`agents/lint.md`**, and prompts updated accordingly.
- **Horde HTTP:** follow-up markdown and managed worker log directories are fixed paths under **`workdir`** defined as **`FOLLOWUP_ARTIFACT_REL`** (`debug/followups/`) and **`AGENTS_LOG_REL`** (`agents_log/`) in [`kowalski/src/horde.rs`](kowalski/src/horde.rs) (not `horde.md` frontmatter).
- **Knowledge Compiler / `agent-app`:** **`main-agent.md` removed** — CLI **`list` / `validate` / `run` / `worker`** now read **`horde.md`** + **`agents/*.md`** (same source as the server). Removed unused **`kc.research`** agent, prompts, and investigation template from the example.
- **Knowledge Compiler / `.gitignore`:** local **`agent-app run`** uses **`examples/knowledge-compiler/output/{PASTE_ME.md,debug/}`** (same default workdir as **`horde.md`**). Ignore rules target **`output/`** only.
- **Knowledge Compiler:** removed optional **external mdBook vault** wiring from `agent_app_ops` (no `external_vault_root` / corpus injection / `EXTERNAL_VAULT_MERGED.md` / `mdbook-summary-suggestion.md`).
- CI: added **`docs`** job (Lychee markdown link check, offline). Local: **`just docs-links`** / `./scripts/docs-linkcheck.sh`.
- Added **`.lychee.toml`**, **`justfile`**, **`scripts/docs-linkcheck.sh`**, root **`LICENSE`** (MIT), and **`CONTRIBUTING.md`**.
- Added docs governance: **`docs/GOVERNANCE.md`** plus governance references in docs index.
- Added architecture snapshots: **`docs/architecture_v02.md`**, **`docs/architecture_v03_future.md`**, and Excalidraw sources under `docs/img/`.
- Consolidated legacy AGENTS content into **`docs/purgatory/legacy_v1.1.0.md`** and replaced inline legacy blocks with pointers.
- **Horde / ACL:** `run_finished.paste_for_obsidian` is renamed to **`handoff_markdown`** (serde still accepts the old JSON key when deserializing). Generic server defaults no longer assume Obsidian or the Knowledge Compiler narrative; per-app copy belongs in each **`horde.md`**.
- **`agent-app` markdown stages:** local and federation **`compile` / `ask` / …** workers share **`kowalski_core::markdown_pipeline`** (`context_paths`, `@artifact@`, `@step:name@`, per-stage **`output`**, optional **`normalize_*`**). Rust no longer runs wiki repair, index rebuild, or **`write_paste_me_file`** — final deliverable shape is whatever the last stage’s prompt writes (the KC example ends with **`PASTE_ME.md`** as `agents/lint.md` `output`). **`horde.md`** **`delivery_*`** fields remain for server/UI copy; the sample manifest dropped unused **`handoff_*`** keys.
- **LLM / operator UX:** Ollama and **OpenAI-compatible** providers now attach the same style of **“what to check”** hints (API base, model, key, network). [`kowalski-core/src/llm/provider.rs`](kowalski-core/src/llm/provider.rs) documents the convention for future **`LLMProvider`** implementations. **`agent-app`** `chat_no_tools` preserves the server error body on failed **`POST /api/chat`**; CLI **`friendly_http_status_error`** adds Ollama-specific hints for HTTP 5xx on `/api/chat`.

## [1.1.0] - 2026-04-30

> Horde-first release line: Knowledge Compiler app workflow, markdown-defined multi-agent orchestration, federation worker/delegate loop, and operator visibility upgrades.

### Added

- **Knowledge Compiler horde workflow** as the first federation-oriented app path (`examples/knowledge-compiler`), including reproducible ingest -> compile -> ask -> lint runs and proof-run instructions.
- **Markdown-defined agent orchestration** (`main-agent.md` + `agents/*.md`) with CLI operators to list, validate, and run app-defined sub-agent pipelines.
- **Federation app execution loop** for delegated horde work: task delegation, worker execution, progress publishing, and structured task results with artifact paths.
- **Operator-facing observability** for federation runs in the Vue UI, including step-by-step task progress events and final artifact delivery visibility.

### Changed

- **Documentation line moved to 1.1.0** across workspace and crate READMEs/roadmaps, with explicit summary of what changed between `1.0.0` and `1.1.0` for the horde workflow.
- **Knowledge Compiler runtime UX** now supports natural-language extension entry, serialized sub-agent execution trace output, and clearer end-of-run artifact reporting.

### Documentation

- Added **`docs/README.md`** (index), **`docs/OVERVIEW_1_1.md`** (1.1.x narrative), and **`docs/purgatory/`** for superseded articles and legacy static HTML.
- Updated **`docs/article_memory.md`**, **`docs/memory_architecture.md`**, **`docs/article_tooling.md`**, and **`docs/key_technology.md`** for **`TemplateAgent`** / **`kowalski-core`** naming and current memory stack wording.
- Corrected **per-crate `ROADMAP.md`** files (version **1.1.0**; CLI vs **`kowalski`** HTTP responsibilities).


## [1.0.0] - 2026-04-12

> First **1.x** line: consolidated crates, operator CLI + Vue UI, MCP (HTTP/SSE + DataFusion server), optional Postgres/pgvector/AGE, federation hooks.

### Added

- **Workspace version 1.0.0** for `kowalski-core`, `kowalski-cli`, `kowalski`, `kowalski-mcp-datafusion`, and the Vue **operator UI** (`ui/`).
- **HTTP API** (`kowalski`): chat, chat stream, tool-aware streaming via **`tools_stream`** on **`POST /api/chat/stream`** (tokens only after tool execution in the same request); graph status and **`POST /api/graph/cypher`** (Apache AGE) when built with **`--features postgres`**.
- **Vue Chat:** checkbox **Tool-aware stream** (`tools_stream`).
- **CI:** `pgvector/pgvector:pg16` for default Postgres tests; **`apache/age:release_PG16_1.6.0`** job for **`postgres_age_cypher`** integration test; **`kowalski-cli`** build with **`postgres`** feature.
- **`kowalski-mcp-datafusion`:** Streamable HTTP MCP server over CSV/Parquet (DataFusion).

### Changed

- **Cargo feature `postgres`:** PostgreSQL + **`pgvector`** are **optional**. The feature enables `sqlx/postgres`, optional **`pgvector`**, and **`pgvector/sqlx`** (vector types for SQLx). Default builds omit them. Enable with `cargo build -p kowalski-core --features postgres`, `cargo build -p kowalski-cli --features postgres`, or `cargo build -p kowalski --features full` (includes CLI + postgres). If `memory.database_url` is `postgres://…` without the feature, configuration returns a clear error.
- **Semantic memory (Tier 3) + Postgres:** When **`memory.database_url`** is **`postgres://…`**, semantic tier uses **`PostgresSemanticStore`** ([`semantic_pg.rs`](kowalski-core/src/memory/semantic_pg.rs)): tables **`semantic_memory`** / **`semantic_relation`** ([`003_semantic_memory.sql`](kowalski-core/migrations/postgres/003_semantic_memory.sql)), **pgvector** cosine distance (`<=>`). **`MemoryProvider::retrieve`** embeds the query via **`LLMProvider`** and runs SQL similarity (with `ILIKE` fallback). New config **`memory.embedding_vector_dimensions`** (default **768**, must match `vector(N)` in the migration). Depends on **`pgvector`** crate. In-process **`SemanticStore`** remains the default when no Postgres URL.
- **Episodic memory (Tier 2):** Replaced **RocksDB** with **`sqlx`** + **`episodic_kv`**: default is a **local SQLite file** under `episodic_path` (directory → `episodic.sqlite`, or an explicit `.sqlite`/`.db`). Optional **`postgres://…`** in `memory.database_url` uses the same JSON rows in **PostgreSQL** (`kowalski-core/migrations/postgres/002_episodic_kv.sql`). **`EpisodicBuffer::open(&MemoryConfig, …)`**, **`Consolidator::new(&MemoryConfig, …)`** (async); **`consolidate`** runs SQL migrations when `database_url` is set.
- **Semantic memory relations:** Replaced **`petgraph`** with a **`HashMap<String, Vec<(String, String)>>`** (subject → outgoing `(predicate, object)` edges). Same behavior for the current query pattern; **one fewer dependency**; no graph crate—only `std`. See [`docs/DESIGN_MEMORY_AND_DEPENDENCIES.md`](docs/DESIGN_MEMORY_AND_DEPENDENCIES.md).

### Documentation

- Documented **memory stack rationale**: **Qdrant** was used in an **initial proof of concept** for semantic memory; the **ongoing goal** is a **simple, robust, dependency-light** default with **minimal moving parts**. Canonical write-up: [`docs/DESIGN_MEMORY_AND_DEPENDENCIES.md`](docs/DESIGN_MEMORY_AND_DEPENDENCIES.md). Linked from root and component `AGENTS.md`, READMEs, memory articles, and rebuild notes.
- Refreshed **README.md**, **AGENTS.md**, **ROADMAP.md** (root and key sub-crates).

[1.5.0]: https://github.com/yarenty/kowalski/compare/v1.3.0...v1.5.0
[1.3.0]: https://github.com/yarenty/kowalski/releases/tag/v1.3.0
[1.2.0]: https://github.com/yarenty/kowalski/releases/tag/1.2.0
[1.1.0]: https://github.com/yarenty/kowalski/releases/tag/1.1.0
[1.0.0]: https://github.com/yarenty/kowalski/releases/tag/1.0.0

## [0.5.2] - 2024-07-06

> "Version 0.5.2: Now with a memory like an elephant (but less likely to trample your data)."

### 🧠 Added
- **Memory Module: The Brain Arrives!**
  - Introduced the shiny new `kowalski-memory` module, giving agents the power to remember, forget, and reminisce about the good old days (i.e., previous prompts).
  - Supports **all 3 types of memory**:
    - **Episodic Memory:** For remembering what just happened ("Did I already answer that?").
    - **Semantic Memory:** For storing facts, concepts, and trivia ("Paris is the capital of France, and so is every other AI's favorite example.").
    - **Working Memory:** For short-term, in-the-moment reasoning ("What was I doing again?").
  - Modular, extensible, and ready for future memory experiments (or existential crises).

### 🏁 Benchmarking: Kowalski vs. LangChain
- **Initial Benchmark Suite:**
  - Added a set of benchmarking scenarios to compare Kowalski's performance and reasoning against LangChain.
  - Benchmarks cover simple LLM calls, tool use, memory retrieval, and CSV analysis.
  - Results logged for both frameworks—let the games (and the bragging) begin!

### 🛠️ Improved
- Documentation updates for the new memory module and benchmarking process.
- Minor bug fixes and performance tweaks (because every release needs a few of these).

## [0.5.1] - 2024-07-01

> "Version 0.5.1: Now with 42% more reactivity and a filesystem that actually listens to you."

### ✨ Added
- **React-Style Tool Processing for All Agents:**
  - Agents now process tool calls in a React-like, stepwise fashion (think: "thought, tool, action, repeat").
  - Enables more dynamic, context-aware, and multi-step reasoning for all agent types.
  - Tool outputs are now seamlessly integrated into agent responses—no more "tool says hi, agent ignores it" moments.

- **New Filesystem Tools (`fs`):**
  - Added a suite of pluggable filesystem tools for reading, writing, listing, and manipulating files and directories.
  - Agents can now interact with the local filesystem in a safe, modular way (no more "accidentally deleted the project" moments... probably).
  - Unified interface for file operations across all agents.

- **Data-Agent: Now Fully Operational:**
  - The `kowalski-data-agent` is now feature-complete and ready for data wrangling.
  - Supports CSV, tabular, and structured data analysis out of the box.
  - Improved error handling, configuration, and extensibility for custom data workflows.

### 🛠️ Improved
- Enhanced agent orchestration logic for better multi-step tool use.
- Unified tool API across all agent modules for easier extension and maintenance.
- Documentation updates for new tools and agent capabilities.

## [0.5.0] - 2024-06-29

> "Version 0.5.0: The Great Kowalski Restructurization. Now with 100% more modules!"

### 🚀 Major Release: Modular Kowalski
- **Project Restructurization:**
  - Split the codebase into clear, separate modules:
    - `kowalski-core`: Foundational types, agent abstractions, conversation, roles, configuration, error handling, and toolchain logic.
    - `kowalski-agent-template`: Flexible agent base, builder, and ready-to-use templates for rapid agent development.
    - `kowalski-tools`: Pluggable tools for code, data, web, and document analysis.
    - `kowalski-federation`: (WIP) Multi-agent orchestration, registry, and federation protocols.
    - Specific agents (e.g., academic, code, data, web) now live in their own crates.
  - Each module now has its own README and clear documentation.
  - Lays the groundwork for future multi-agent, federated, and plugin-based development.

### 🏗️ Architecture
- **Separation of Concerns:** Each module is now responsible for a single aspect of the system, making the codebase easier to maintain, extend, and test.
- **Extensibility:** New agents, tools, and federation protocols can be added without touching the core logic.
- **Documentation:** All major modules now have comprehensive, modern README files.

### 🧪 Federation (Experimental)
- Initial implementation of `kowalski-federation` for multi-agent orchestration.
- Open questions remain about protocol selection (A2A, ACP, MCP, or custom).
- Marked as UNDER CONSTRUCTION—expect rapid changes and design discussions.

### 🧰 Tools
- All tools (code, data, web, document) are now in `kowalski-tools` and can be plugged into any agent.

### 🧑‍💻 Agent Templates
- `kowalski-agent-template` provides a builder and templates for fast custom agent creation.
- General and research agent templates included as examples.

### 🗃️ Other
- Updated and unified dependency management across all modules.
- Improved test coverage and modular test structure.
- Cleaned up legacy code, TODOs, and dead ends from previous versions.

---

## [0.3.0] - 2024-03-10

> "Version 0.3.0: Because 0.2.0 wasn't confusing enough." - A Version Control Enthusiast

### 🎭 Added
- **CLI Interface** (because typing commands is more fun than clicking buttons):
  - `kowalski chat`: Talk to your AI without all the fancy UI
  - `kowalski academic`: Analyze papers without actually reading them
  - `kowalski model`: Manage your AI models like a pro
  - Command-line arguments that make sense (for once)
  - Helpful error messages (they're still errors, but at least they're helpful)

### 🔧 Changed
- Completely revamped command-line interface (it's not just a bunch of flags anymore)
- Improved model management commands (your AI models are now properly domesticated)
- Enhanced error handling in CLI (because users deserve to know what they did wrong)
- Better streaming response handling (watch your AI think in real-time, now with better formatting)

### 🐛 Fixed
- CLI argument parsing issues (now it actually understands what you're trying to say)
- Model management command errors (your models won't disappear into the void anymore)
- Response streaming formatting (no more broken lines or missing characters)
- Various "it works on my machine" issues (it still might not work on yours, but at least we tried)

### 📚 Documentation
- Added CLI usage examples (because reading the code is so last year)
- Updated README with command-line instructions (they're actually useful this time)
- Added command help messages (they're sarcastic, but they work)
- Improved error messages (they're still errors, but at least they're funny)

### 🔬 Technical Debt
- Replaced quick CLI hacks with slightly more sophisticated CLI hacks
- Moved CLI-related TODOs to actual GitHub issues
- Pretended to understand command-line argument parsing better

### 🎯 Dependencies
- Added `clap` for proper CLI argument parsing (because parsing strings manually is so 2010)
- Updated other dependencies (because old code is like old milk - it smells bad)
- Removed deprecated dependencies (they served us well, but it's time to move on)

## [0.2.0] - 2024-03-09

> "The best time to write a changelog is when you make the changes. The second best time is right before a release when you've forgotten everything you did." - Ancient Developer Proverb

### 🎭 Added
- **New Agents** (because one AI personality wasn't enough):
  - `GeneralAgent`: Your friendly neighborhood AI with a dash of sass
  - `ToolingAgent`: The Swiss Army knife of web research
  - `AcademicAgent`: The one that actually reads the papers

- **Tool System** (like Batman's utility belt, but for AI):
  - Web browsing capabilities (because opening Chrome is too mainstream)
  - DuckDuckGo integration (Google who?)
  - HTML parsing with multiple fallback strategies
  - Dynamic content handling (JavaScript can't hide from us anymore)
  - Rate limiting (to avoid angry emails from server admins)

- **Examples** (because documentation is better with code):
  - `model_manager`: Herding your AI models like cats
  - `academic_research`: Making research papers readable again
  - `web_research`: Like having a very fast research assistant
  - `web_search`: For when typing in a browser is too much work
  - `web_dynamic`: Handling modern web apps like a pro
  - `web_static`: Old-school HTML scraping
  - `general_chat`: When you just want to chat with a sarcastic AI

### 🔧 Changed
- Completely revamped agent architecture (it's not spaghetti code anymore, we promise)
- Improved conversation management (your AI won't forget things... as often)
- Enhanced streaming responses (watch your AI think in real-time)
- Better error handling (because things will go wrong, we just handle it better now)

### 🐛 Fixed
- Memory leaks in conversation handling (your RAM can thank us later)
- Race conditions in async operations (time is now properly wibbly-wobbly)
- Various "it works on my machine" issues
- That one bug that nobody could reproduce but everyone complained about

### 📚 Documentation
- Added sarcastic comments throughout the codebase
- Created actually useful examples (a rare achievement)
- Updated README with proper setup instructions
- Added this CHANGELOG (because git log was getting boring)

### 🔬 Technical Debt
- Replaced quick hacks with slightly more sophisticated hacks
- Moved TODOs to actual GitHub issues
- Pretended to understand async/await better

### 🎯 Dependencies
- Updated all the things (except the ones that would break everything)
- Added more crates (because why solve problems yourself?)
- Removed deprecated dependencies (they served us well)

## [0.1.0] - 2024-03-07

### Added
- Initial release
- Basic Ollama integration
- Proof that we could make it work
- A lot of hopes and dreams

> "Change is inevitable, except from a vending machine." - Robert C. Gallagher

> "Version numbers are like birthdays - they keep increasing but nothing really changes." - A Cynical Developer

### Added
- Basic agent functionality (because talking to machines wasn't complicated enough)
- Multiple model support (because one AI model isn't confusing enough)
- Conversation management (like herding cats, but with more JSON)
- Role-based interactions (giving AI personalities, what could go wrong?)
- PDF and text file support (because copy-pasting was too mainstream)
- Streaming responses (watch your AI think in real-time, it's like watching paint dry but more expensive)
- Configuration system (because hardcoding values is too simple)
- Error handling (because we're optimists who plan for the worst)

### Features
- Implemented `Agent` struct (it's like a digital pet, but less cuddly)
- Added `ModelManager` for handling Ollama models (your personal AI zookeeper)
- Created `Role`, `Audience`, and `Preset` enums (because we love pretending our AI has a personality)
- Added `PdfReader` and `PaperCleaner` utilities (because PDFs are like onions - they have layers and make you cry)
- Implemented conversation history (because AIs need memories too)
- Added streaming support (for those who enjoy watching their CPU melt in real-time)

### Technical Debt
- "TODO" comments that will definitely be addressed in the next version (narrator: they won't)
- Some magic numbers that seemed like a good idea at 3 AM
- Documentation that assumes the reader can read minds
- Error messages that are more cryptic than your ex's texts

### Known Issues
- Sometimes the AI gets philosophical (we're working on reducing its exposure to existential literature)
- Configuration files multiply like rabbits in the wrong directory
- Error messages occasionally include Shakespeare quotes (we suspect the AI is going through a literature phase)
- The code works (this is suspicious and under investigation)

> "It's not a bug, it's an undocumented feature." - Anonymous  
> "The code is more what you'd call 'guidelines' than actual rules." - Pirates of the Caribbean, probably

### Dependencies
- Added every crate that looked interesting on crates.io
- Removed half of them because they were causing conflicts
- Added them back because the errors were worse
- Settled on a set that mostly works (fingers crossed)

### Documentation
- Added comments that range from "obviously redundant" to "cryptically useless"
- Created a README that nobody will read
- Added docstrings that are more entertaining than informative
- Included examples that work 60% of the time, every time

> "Documentation is like true love - it exists, but it's hard to find." - A Documentation Writer
> "The only thing worse than no documentation is wrong documentation." - A Frustrated Developer

[0.5.2]: https://github.com/yarenty/kowalski/releases/tag/0.5.2 
[0.5.1]: https://github.com/yarenty/kowalski/releases/tag/0.5.1 
[0.5.0]: https://github.com/yarenty/kowalski/releases/tag/0.5.0 
[0.2.0]: https://github.com/yarenty/kowalski/releases/tag/0.2.0 
[0.1.0]: https://github.com/yarenty/kowalski/releases/tag/0.1.0 
