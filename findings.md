# Findings

## Repository observations

- `examples/knowledge-compiler` is the primary **Knowledge Compiler** horde scaffold (agents, prompts, templates, `horde.md`).
- Root planning files (`task_plan.md`, `findings.md`, `progress.md`) track long-horizon goals; numbered phases in `task_plan.md` now extend through **26** (21–22 delivered, 23–26 planned).

## Product framing

- "Knowledge Compiler" best matches the ingest -> compile -> maintain -> query workflow.
- This maps well to Kowalski's `TemplateAgent` + toolchain architecture.

## Implementation findings

- Portable shell scripts should avoid `xargs` in this environment; loop-based rendering is safer.
- The scaffold can already run a full dry pipeline and generate markdown outputs.
- Native CLI integration is straightforward via a dedicated command family in `main.rs` plus one focused ops module.
- The best long-term boundary is generic `extension` orchestration in CLI and domain logic in extension runners.
- Federation app usability benefits from a generic `/api/federation/publish` endpoint for `task_result` and custom ACL events.
- Agent-driven compile/ask/lint can be implemented cleanly by composing prompt files + file context into `/api/chat` requests from the extension runner.
- `POST /api/chat` needed a no-tools mode for app determinism; tool loops can inject unrelated tool errors into domain outputs.
- Even with no-tools mode, runtime guardrails are needed because some model responses may still be `{}` or empty.
- AGENTS.md-style orchestration works best with markdown frontmatter definitions (`horde.md`, `agents/*.md`) validated by CLI before execution.
- Obsidian-grade consistency needs post-generation repair passes: filename normalization, wikilink-driven concept stubs, backlink enforcement, and index regeneration.
- Federation app flow can remain markdown-defined by routing delegates to a single `kc.run:<source>|<question>` instruction handled by `agent-app worker`.
- A dedicated `proof` command improves operability by providing preflight checks and exact reproducible multi-terminal commands.
- User needs are now UX-centric: one natural command phrase, explicit multi-task decomposition, visible serialized sub-agent execution, and final Obsidian artifact confirmation.
- Federation UI already had delegate + raw stream primitives; adding worker-published `task_progress` events enabled a clean chat-like step timeline without introducing a new backend endpoint.

- Docs release alignment task: all README files plus root `CHANGELOG.md`/`ROADMAP.md` now describe the 1.2.0 line and summarize horde changes since 1.0.0.

## Phase-2 baseline findings (2026-04-30)

- `examples/knowledge-compiler` runtime hygiene is largely complete: legacy top-level runtime dirs are removed and only `output/` remains for generated artifacts.
- Current directory layout is: `README.md`, `agents/`, `horde.md`, `output/`, `prompts/`, `templates/`.

## Phase-1 baseline findings (2026-04-30)

- `cargo deny` currently fails without explicit config due to strict default license policy and multiple advisories.
- There is no existing `deny.toml` in repo root; policy needs to be introduced and versioned.
- Some advisories are fixable via dependency updates (`rustls-webpki`, `tracing-subscriber`, `rand` ranges), while others are currently upstream/transitive and require explicit temporary exceptions with rationale.
- Implemented baseline: `deny.toml` added, fixable advisories updated, and unavoidable transitive advisories captured in explicit ignore list.
- License policy required explicit allows for `Apache-2.0 WITH LLVM-exception`, `BSL-1.0`, and `bzip2-1.0.6` due transitive dependencies.
- Final validation: `cargo deny check` now passes (`advisories ok, bans ok, licenses ok, sources ok`).

## Planned delivery phases 21–26 (2026-05-03)

| Phase | Intent | Status |
| --- | --- | --- |
| 21 | KC `AGENTS.md` / README / `agents/ingest.md`: horde layout, `GITHUB_TOKEN`, explicit Obsidian = copy/sync operator step | Done |
| 22 | Core `source_bundle` + `internal::{github,web,file_system}`: GitHub ingest vs web fetch, bounded local file read, CLI stays thin | Done |
| 23 | Optional mdBook merge: manifest roots, capped corpus from external `doc/**/*.md`, `SUMMARY.md` suggest vs manual | Not started |
| 24 | Relationships: `extends` / `see_also` in templates/prompts, broader wikilink repair, reciprocal link checks | Not started |
| 25 | Research seed / `kc.research` | Dropped from KC example (was unused); revisit only if product wants it back |
| 26 | Operator docs: Docker MCP / external MCP vs small internal tools; optional HTML extraction story | Not started |

## Ingest architecture (2026-05-03)

- **Bundling** for federation / `agent-app` workers uses **`kowalski_core::source_bundle`**, not `kowalski-cli`-local modules, so the same behavior applies regardless of entrypoint.
- **URL routing:** `resolve_github_fetch` gates GitHub-specific fetch (`fetch_url_for_ingest`); on failure (or after non-match), **`fetch_url_as_markdown`** handles generic HTTP(S) with HTML→markdown heuristics.
- **Local files:** **`read_file_bounded`** with **`DEFAULT_MAX_READ_BYTES`** avoids unbounded memory on large paths referenced in ingest input.
- **Three-way tool direction (product):** in-repo MCP where appropriate, external MCP (e.g. catalog gateways) for vendor/browser flows, **`tools/internal/*`** for small synchronous helpers—swap via config later, not by scattering horde logic in CLI.

## Obsidian / vault reality check

- Kowalski does **not** call Obsidian’s APIs. **Paste-first path:** **`PASTE_ME.md`** at horde `workdir` root plus **`run_finished.paste_for_obsidian`**. Intermediates under **`debug/`**.

## Phase 23 completion (2026-05-03)

- Evolved from vault-folder README toward **paste-first** for high-frequency runs; see `CHANGELOG` [Unreleased] and `examples/knowledge-compiler/AGENTS.md`.
