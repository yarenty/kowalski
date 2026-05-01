# Documentation Governance

This page defines how documentation is managed in this repository.

## Sources of truth

- **Operational docs:** root [`README.md`](../README.md), root [`CHANGELOG.md`](../CHANGELOG.md), root [`ROADMAP.md`](../ROADMAP.md).
- **Component execution rules:** crate/package `AGENTS.md` files.
- **Design narratives:** files under [`docs/`](./README.md).
- **Historical docs:** [`docs/purgatory/`](./purgatory/README.md).

## Mandatory update rule

For any refactor, API change, CLI flag change, or behavior change:

1. Update affected `README.md` / `AGENTS.md`.
2. Update `CHANGELOG.md` when user-visible.
3. Update `docs/` if architecture/operator workflow changed.
4. Keep the documentation update in the **same PR** (or immediately stacked follow-up).

This is also codified as **Rule 7** in root [`AGENTS.md`](../AGENTS.md).

## Link quality and CI

- CI runs Lychee over `**/*.md` in offline mode.
- Local validation:

```bash
just docs-links
# fallback:
./scripts/docs-linkcheck.sh
```

## Architecture assets process

- Source diagrams are versioned under [`docs/img/`](./img/), e.g. `architecture_v02.excalidraw`.
- Add a short companion markdown note when introducing a new architecture version.
- Keep one “current” and one “future” architecture asset for roadmap conversations.
