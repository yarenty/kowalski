# Portable workflow manifest — JSON interchange for hordes

Kowalski hordes are **markdown directories** (`horde.md` + `agents/*.md` + `prompts/*.md`):
great for humans, editors, and git — not for interchange. The **workflow manifest** is the
canonical **JSON** representation of the same workflow, so hordes can be exported, imported,
validated, and consumed by other systems. The markdown dir stays the **authoring** format;
the manifest is **interchange/runtime only** — import always regenerates a horde directory
before anything runs.

## The contract

- Rust model: `kowalski_core::manifest::WorkflowManifest` — strict serde with
  `deny_unknown_fields` on every struct; a manifest either matches the contract exactly or
  fails to parse.
- Published JSON Schema (draft 2020-12):
  [`kowalski-core/resources/schemas/workflow-manifest.schema.json`](../kowalski-core/resources/schemas/workflow-manifest.schema.json)
  (embedded in the crate as `WORKFLOW_MANIFEST_SCHEMA_JSON`).
- Versioning: `schema_version` is `MAJOR.MINOR` (current `1.0`). A different MAJOR is
  rejected by validation; unknown step kinds are a portability concern reported as
  warnings, never a crash.

Top level: `schema_version`, kebab-case `identifier`, `name`, semver `version`,
`description`, `steps[]`, `pipeline[]` (topological order), `edges[]` (scheduling DAG,
1:1 with the horde `[[edges]]` declaration — `{from, to, when: pass|fail|always,
max_loops}`), and a `kowalski` extension block.

Per step: `id`, `kind` (schema enum: kowalski LLM kinds `process`/`step`/`deliver`/
`final`/`compile`/`ask`/`lint`, deterministic kinds `verify`/`apply`/`ingest`, plus the
interchange vocabulary `llm`/`rag`/`input`), `name`, `description`,
`agent {system_prompt, model?, parameters?}` (prompt files are **inlined** into
`system_prompt` on export), `tool_bindings[] {provider, tools?, overrides?}` (flat
kowalski `tool_ids` export under provider `builtin`; never credentials),
`input {prompt, schema?}`, and a free-form `ui` block — kowalski's convention is
`{"inputs": [...]}` carrying the `[[inputs]]` pre-run operator form verbatim.

**Extension blocks.** Everything horde-specific with no generic manifest home rides the
`kowalski` object — at manifest level: capability prefix, run defaults, delivery
presentation, and `triggers[]` with **every** declaration field (including `overlap`);
at step level: `output`, `context_paths`, verify/apply config, isolation, avatar, and the
agent markdown body. Export resolves writer defaults first, so a manifest is
self-contained.

**Never exported:** credentials, run history, run document contents, and server-local
operator state (e.g. trigger enable/disable overrides).

## Converters and round-trip guarantees

```text
horde dir  ──  horde_dir_to_manifest()  ──▶  WorkflowManifest (JSON)
horde dir  ◀──  write_manifest_tree()   ──   (validates at Publish strictness,
                                              then the rookery writer regenerates the tree)
```

- **dir → manifest → dir preserves semantics**: pipeline, DAG edges (exactly, including
  conditional `when` and `max_loops` loop-backs), prompts, operator inputs, tool ids,
  triggers, verify/apply config, and normalization settings all survive; tested against
  `examples/coder` and `examples/knowledge-compiler`.
- **manifest → dir → manifest is byte-stable** modulo formatting (serialized JSON
  equality on re-export).

Validation is two-tier via `validate_manifest_with`: `Draft` enforces structural
integrity only (ids, coverage, edges, kind-config exclusivity); `Publish` additionally
requires a name, a semver version, and a non-empty `agent.system_prompt` on every LLM
step.

Archive packaging (zip bundle + import endpoint) and CLI/UI surfaces build on this
contract — see the root [`ROADMAP.md`](../ROADMAP.md).
