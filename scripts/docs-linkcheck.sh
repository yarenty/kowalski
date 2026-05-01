#!/usr/bin/env bash
# Validate markdown links (repo-relative files and anchors). HTTP(S) URLs are
# skipped unless you omit --offline (see .lychee.toml).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v lychee >/dev/null 2>&1; then
  echo "lychee not found. Install with: cargo install lychee" >&2
  exit 1
fi

exec lychee --config .lychee.toml --offline --no-progress './**/*.md'
