# Contributing

Open a **pull request** or **issue** on [GitHub](https://github.com/yarenty/kowalski). For larger changes, describe the problem and the approach in the issue first when it helps review.

- Run **`cargo clippy`** and **`cargo test`** for crates you touch.
- Run **`just docs-links`** (or `./scripts/docs-linkcheck.sh`) if you change Markdown under the repo.
- **Rule 7 ([`AGENTS.md`](./AGENTS.md)):** refactors and behavior changes are **not complete** until **`AGENTS.md`**, **`README.md`**, and **`CHANGELOG.md`** (when user-visible) are updated in the same PR or stacked immediately after.
- Keep commits focused; update related docs and tests with code changes.

## Releasing

1. `./scripts/release-version.sh bump X.Y.Z` — sets the workspace version and every mirror
   (internal dependency versions, `ui/package.json`, install examples). Crate READMEs carry no
   version, so there is nothing to edit there.
2. Write the `CHANGELOG.md` section (and the `ROADMAP.md` / `AGENTS.md` release lines for a minor
   or major release), open a PR; CI runs `./scripts/release-version.sh check`.
3. After the merge, tag `vX.Y.Z` on `main` and push it — the Release workflow builds the binaries
   that `install.sh` downloads.
4. Publish the crates from a clean checkout of the tag: `./scripts/publish-crates.sh`.
