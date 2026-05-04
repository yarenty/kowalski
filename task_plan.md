# Task Plan: Knowledge Compiler Example

## Goal

Create a production-usable first app (`knowledge-compiler`) on top of Kowalski federation that performs real web ingest, Obsidian-ready markdown compilation, and reliable agent-driven outputs.

## Phases

| Phase | Status | Notes |
| --- | --- | --- |
| 1. Initialize planning files | complete | Created `task_plan.md`, `findings.md`, `progress.md` |
| 2. Design example structure | complete | Defined docs, scripts, prompts, templates, and config contracts |
| 3. Create example files | complete | Added full `examples/knowledge-compiler` scaffold |
| 4. Validate consistency | complete | Ran smoke tests and fixed shell portability issue |
| 5. Integrate with Kowalski CLI | complete | Added native `knowledge-compiler` subcommands to `kowalski-cli` |
| 6. Refactor to extension pattern | complete | Replaced built-in domain commands with generic extension loader |
| 7. Federation-first app workflow | complete | Added federated worker commands + generic federation publish API endpoint |
| 8. Agent-driven compile/query/lint | complete | Replaced placeholder scripts with `/api/chat` TemplateAgent calls in extension runner |
| 9. Stabilize generation path | complete | Added `use_tools` control in `/api/chat` and enforced no-tools extension calls |
| 10. Real web ingest | complete | Added URL fetch + HTML extraction into normalized markdown source files |
| 11. Obsidian output hardening | complete | Added schema normalization, concept filename normalization, backlink repair, and index refresh |
| 12. Markdown-defined agent system | complete | `horde.md` + `agents/*.md`; CLI validation/run/list (was `main-agent.md`; unified) |
| 13. Remove shell orchestration logic | complete | Shell extension reduced to thin wrapper over CLI `agent-app` engine |
| 14. End-to-end federation validation | complete | Added `agent-app` federation delegate/worker flow with structured task results and artifact paths |
| 15. UX command + task trace visibility | complete | Added natural-language extension entrypoint, explicit per-subagent task trace, and final artifact summary |
| 16. UI federation run observability | complete | Added chat-like UI runner, live sub-agent task progress events, and final artifact delivery in federation panel |
| 17. Dependency/Security baseline (`cargo deny`) | complete | Added repo `deny.toml`, upgraded fixable vulnerable dependencies, and codified explicit temporary exceptions for unavoidable transitive advisories |
| 18. Runtime artifact hygiene validation | complete | Verified only `output/*` remains as runtime tree and confirmed `.gitignore` shields generated output |
| 19. Horde UX polish | complete | Added `workdir` + `clean_on_startup` visibility in Horde Management/Run panels and output-folder quick action |
| 20. Multi-URL ingest traceability | complete | Ingest artifact now includes source metadata table and per-source begin/end section markers for URLs/files/text |
| 21. KC operator docs + vault boundary | complete | `examples/knowledge-compiler/AGENTS.md`, README links; horde `workdir`, `GITHUB_TOKEN`, explicit “no Obsidian API—folder sync is operator-owned” in `agents/ingest.md` |
| 22. Core ingest + internal FS helpers | complete | `kowalski_core::source_bundle` (not CLI): GitHub URL path via `fetch_url_for_ingest` with fallback to `fetch_url_as_markdown`; non-GitHub HTTP(S) uses web path; local paths use `tools::internal::file_system::read_file_bounded`; re-exports in `tools/internal/mod.rs` |
| 23. Optional mdBook / external vault merge | superseded | Removed from product path: no `external_vault_root` or vault merge artifacts; KC uses ingest → wiki only. |
| 24. Article relationships + link hygiene | pending | Templates/prompts for `extends` / `see_also`; widen wikilink repair beyond summaries; lint or prompt-check reciprocal links |
| 25. Research seed + investigation flows | dropped (KC) | Removed from example; re-open only if product wants `kc.research` again |
| 26. Optional MCP / Docker operator docs | pending | Document Docker MCP Toolkit / external MCP vs small `internal/*` tools; headless HTML extraction options without bloating core |

## Decisions

- Use the name **Knowledge Compiler** as primary branding.
- Keep implementation local-first and markdown-first.
- Provide shell scripts for quick start without requiring immediate Rust code changes.
- Add first-class CLI operators so workflows run through `kowalski-cli` directly.
- Keep domain workflows out of core CLI by using extension dispatch.
- Expose generic federation message publishing endpoint for app-level result reporting.
- Prefer deterministic no-tools generation for compiler/query/lint unless explicit tool usage is required.
- Agent orchestration definitions must be markdown-first, visible, and validated against declared available agent names.
- Runtime artifact root is horde-level configurable and currently standardized under `examples/knowledge-compiler/output`.
- Next hardening phase is split into security baseline, runtime hygiene, UX transparency, and multi-URL traceability.
- **Ingest and fetch rules live in `kowalski-core`** (`source_bundle`, `tools::internal::{github,web,file_system}`); **`kowalski-cli` / `ui/` stay thin** executors—no horde-specific fetch logic in CLI.
- **Obsidian delivery** remains markdown-on-disk under horde `workdir/`; full vault import and mdBook `SUMMARY.md` automation are tracked in phases 23–26, not assumed complete.

## Errors Encountered

| Error | Attempt | Resolution | Lessons Learned |
| --- | ---: | --- | --- |
| `task_plan.md` not found | 1 | Created new planning files in repo root | Session state can differ from earlier git snapshots |
| `xargs: sysconf(_SC_ARG_MAX) failed` in `compile.sh` | 1 | Replaced `xargs` path rendering with portable shell loops | Prefer robust loops in cross-platform shell scripts |
| `publish` type mismatch in federation API | 1 | Passed `&AclEnvelope` instead of owned value | Check method signatures when adding new API handlers |
| Model returned `{}` for app prompts | 1 | Added no-tools path + markdown fallbacks in extension runner | Guardrails are required even with valid API responses |
| Full `cargo clippy --fix --all-features` failed initially | 1 | Ran fixes outside sandbox and corrected `kowalski` optional `cli` dependency wiring | Feature-gated exports must map to optional dependencies in crate manifests |
