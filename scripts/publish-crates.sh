#!/usr/bin/env bash
# Publish Kowalski workspace crates to crates.io in dependency order.
#
# Usage:
#   ./scripts/publish-crates.sh              # checks + publish all
#   ./scripts/publish-crates.sh --dry-run    # build + package (skips crates whose deps are not on crates.io yet)
#   ./scripts/publish-crates.sh --from kowalski-cli   # resume mid-sequence
#   ./scripts/publish-crates.sh --skip-checks         # publish only
#   ./scripts/publish-crates.sh --allow-dirty         # allow uncommitted tree
#
# Prerequisites:
#   cargo login   # once, with a crates.io API token
#   You must own each crate name on crates.io (first publish claims it).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Dependency order: each crate's path deps must already be on the index.
CRATES=(
  kowalski-mcp-base
  kowalski-core
  kowalski-cli
  kowalski-mcp-datafusion
  kowalski-mcp-rookery
  kowalski
)

DRY_RUN=0
SKIP_CHECKS=0
ALLOW_DIRTY=0
FROM_CRATE=""

usage() {
  sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
  exit "${1:-0}"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1 ;;
    --skip-checks) SKIP_CHECKS=1 ;;
    --allow-dirty) ALLOW_DIRTY=1 ;;
    --from)
      shift
      FROM_CRATE="${1:-}"
      [[ -n "$FROM_CRATE" ]] || usage 1
      ;;
    -h | --help) usage 0 ;;
    *)
      echo "Unknown option: $1" >&2
      usage 1
      ;;
  esac
  shift
done

cargo_args=()
[[ "$ALLOW_DIRTY" -eq 1 ]] && cargo_args+=(--allow-dirty)

crate_in_list() {
  local needle="$1"
  local c
  for c in "${CRATES[@]}"; do
    [[ "$c" == "$needle" ]] && return 0
  done
  return 1
}

run_checks() {
  echo "==> cargo build (default-members)"
  cargo build

  echo "==> cargo test -p kowalski-core -p kowalski-cli -p kowalski-mcp-base"
  cargo test -p kowalski-core -p kowalski-cli -p kowalski-mcp-base

  if command -v cargo-deny >/dev/null 2>&1; then
    echo "==> cargo deny check licenses"
    cargo deny check licenses
  else
    echo "WARN: cargo-deny not installed; skipping license check (cargo install cargo-deny)" >&2
  fi
}

package_crate() {
  local crate="$1"
  local extra=()
  [[ "$DRY_RUN" -eq 1 ]] && extra+=(--no-verify)
  echo "==> cargo package -p ${crate} ${cargo_args[*]:-} ${extra[*]:-}"
  # shellcheck disable=SC2086
  if ! cargo package -p "$crate" ${cargo_args[@]+"${cargo_args[@]}"} ${extra[@]+"${extra[@]}"}; then
    if [[ "$DRY_RUN" -eq 1 ]]; then
      echo "WARN: skipped packaging ${crate} — path deps must be on crates.io first (expected after earlier publishes)." >&2
      return 0
    fi
    return 1
  fi
}

publish_crate() {
  local crate="$1"
  echo "==> cargo publish -p ${crate} ${cargo_args[*]:-}"
  # shellcheck disable=SC2086
  # cargo publish already blocks until the crate is visible on the registry index.
  cargo publish -p "$crate" ${cargo_args[@]+"${cargo_args[@]}"}
}

if [[ -n "$FROM_CRATE" ]]; then
  crate_in_list "$FROM_CRATE" || {
    echo "Unknown crate for --from: $FROM_CRATE" >&2
    exit 1
  }
fi

selected=()
if [[ -z "$FROM_CRATE" ]]; then
  selected=("${CRATES[@]}")
else
  started=0
  for c in "${CRATES[@]}"; do
    [[ "$c" == "$FROM_CRATE" ]] && started=1
    if [[ "$started" -eq 1 ]]; then
      selected+=("$c")
    fi
  done
fi

if [[ ${#selected[@]} -eq 0 ]]; then
  echo "No crates selected to publish." >&2
  exit 1
fi

echo "Publish sequence: ${selected[*]}"
[[ "$DRY_RUN" -eq 1 ]] && echo "(dry-run: package only, no upload)"

if [[ "$SKIP_CHECKS" -eq 0 && "$DRY_RUN" -eq 0 ]]; then
  run_checks
elif [[ "$SKIP_CHECKS" -eq 0 ]]; then
  echo "==> cargo build (dry-run sanity)"
  cargo build
fi

for crate in "${selected[@]}"; do
  if [[ "$DRY_RUN" -eq 1 ]]; then
    package_crate "$crate"
  else
    publish_crate "$crate"
  fi
done

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "Dry-run complete. Re-run without --dry-run to publish."
else
  echo "Published: ${selected[*]}"
fi
