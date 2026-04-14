#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_INPUT="${1:-}"

if [[ -z "${SOURCE_INPUT}" ]]; then
  echo "Usage: bash scripts/ingest.sh <url-or-short-title>"
  exit 1
fi

mkdir -p "${ROOT_DIR}/raw/sources"
STAMP="$(date +%Y%m%d-%H%M%S)"
SLUG="$(echo "${SOURCE_INPUT}" | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9' '-')"
OUT_FILE="${ROOT_DIR}/raw/sources/${STAMP}-${SLUG}.md"

cat > "${OUT_FILE}" <<EOF
# Raw Source

- Input: ${SOURCE_INPUT}
- Ingested At: $(date -u +"%Y-%m-%dT%H:%M:%SZ")

## Notes
Add normalized source content here (or replace this via an MCP/web-ingest tool).
EOF

echo "Created source stub: ${OUT_FILE}"
