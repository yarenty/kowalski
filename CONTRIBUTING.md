# Contributing

Open a **pull request** or **issue** on [GitHub](https://github.com/yarenty/kowalski). For larger changes, describe the problem and the approach in the issue first when it helps review.

- Run **`cargo clippy`** and **`cargo test`** for crates you touch.
- Run **`just docs-links`** (or `./scripts/docs-linkcheck.sh`) if you change Markdown under the repo.
- **Rule 7 ([`AGENTS.md`](./AGENTS.md)):** refactors and behavior changes are **not complete** until **`AGENTS.md`**, **`README.md`**, and **`CHANGELOG.md`** (when user-visible) are updated in the same PR or stacked immediately after.
- Keep commits focused; update related docs and tests with code changes.
