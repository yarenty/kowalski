#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WIKI_DIR="${ROOT_DIR}/wiki"
OUT_FILE="${ROOT_DIR}/derived/lint/latest.md"

mkdir -p "${ROOT_DIR}/derived/lint"

concept_count="$(ls -1 "${WIKI_DIR}/concepts"/*.md 2>/dev/null | wc -l | tr -d ' ')"
summary_count="$(ls -1 "${WIKI_DIR}/summaries"/*.md 2>/dev/null | wc -l | tr -d ' ')"
broken_links="0"

cat > "${OUT_FILE}" <<EOF
# Knowledge Lint Report

## Snapshot
- Concept pages: ${concept_count}
- Summary pages: ${summary_count}
- Broken links detected: ${broken_links}

## Issues
- Placeholder lint implementation; integrate \`prompts/lint.md\` with a Kowalski lint agent.

## Suggested Fixes
- Add link integrity check against actual wiki filenames.
- Add duplicate concept detection by normalized title.
EOF

echo "Wrote lint report: ${OUT_FILE}"
