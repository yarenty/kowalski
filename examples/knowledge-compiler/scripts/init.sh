#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

mkdir -p "${ROOT_DIR}/raw/sources"
mkdir -p "${ROOT_DIR}/raw/images"
mkdir -p "${ROOT_DIR}/wiki/concepts"
mkdir -p "${ROOT_DIR}/wiki/summaries"
mkdir -p "${ROOT_DIR}/derived/reports"
mkdir -p "${ROOT_DIR}/derived/slides"
mkdir -p "${ROOT_DIR}/derived/lint"
mkdir -p "${ROOT_DIR}/scratch"

if [[ ! -f "${ROOT_DIR}/wiki/index.md" ]]; then
  cat > "${ROOT_DIR}/wiki/index.md" <<'EOF'
# Knowledge Compiler Index

## Concepts
- (none yet)

## Source Summaries
- (none yet)
EOF
fi

echo "Initialized Knowledge Compiler workspace at: ${ROOT_DIR}"
