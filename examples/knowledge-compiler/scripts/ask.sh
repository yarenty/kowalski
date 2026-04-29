#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
QUESTION="${1:-}"

if [[ -z "${QUESTION}" ]]; then
  echo "Usage: bash scripts/ask.sh \"<question>\""
  exit 1
fi

mkdir -p "${ROOT_DIR}/derived/reports"
STAMP="$(date +%Y%m%d-%H%M%S)"
OUT_FILE="${ROOT_DIR}/derived/reports/${STAMP}-answer.md"

cat > "${OUT_FILE}" <<EOF
# Knowledge Compiler Answer

## Question
${QUESTION}

## Response
This is a scaffold response file. Replace this script body with a Kowalski query agent call over \`wiki/\`.

## Sources Used
- [[knowledge-compiler]]
- [[index]]
EOF

echo "Wrote report: ${OUT_FILE}"
