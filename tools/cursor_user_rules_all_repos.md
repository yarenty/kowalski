# Paste into Cursor: global User Rules (all repositories)

Cursor applies **User Rules** on every Agent chat in **every** project. They are not stored in git.

## One-time setup

1. Open **Cursor Settings** → **Rules** (or **Rules, Commands**).
2. Under **User Rules**, paste the block below (or merge with your existing text).
3. Save.

## Text to paste (global)

```markdown
## Defaults for every repository

- **Docs first:** If the repo (or the directory you are editing) has **`AGENTS.md`**, **`CONTRIBUTING.md`**, or a stated architecture doc, read the relevant section before large edits. Prefer extending existing patterns over inventing new ones.
- **SOLID and scope:** Single responsibility—put code where it belongs; prefer new small modules over widening unrelated files; dependency inversion (inject/config) over hard-coded collaborators; no drive-by refactors outside the requested task.
- **Plans:** If **`task_plan.md`**, **`task.md`**, **`docs/ROADMAP.md`**, or similar exists and the task is multi-step, align with it; when you finish a planned slice, update the repo’s progress/log file if one exists (e.g. `progress.md`, `CHANGELOG.md` per project convention).
- **Quality:** Run the project’s tests or build after substantive changes; fix regressions you introduce; do not guess public APIs—open the code or official docs.
- **Honesty:** State uncertainty plainly; ask before breaking compatibility or doing large renames.
```

## Optional: reuse Kowalski’s project rules elsewhere

Cursor can **import rules from GitHub** (Settings → Rules → Remote rule). Point at a repo/path that contains `.cursor/rules/*.mdc`, or copy `.cursor/rules/` into another project.
