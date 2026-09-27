#!/usr/bin/env bash
# Keep the release version consistent everywhere it is written, and keep crate READMEs
# publishable (they are the crates.io and docs.rs pages).
#
# Usage:
#   ./scripts/release-version.sh check          # fail on any drift (CI and publish-crates.sh run this)
#   ./scripts/release-version.sh bump 2.4.0     # set every version mention, then check
#
# The version's owner is [workspace.package] version in the root Cargo.toml. Mirrors:
#   root Cargo.toml [workspace.dependencies] kowalski-core / kowalski-cli
#   ui/package.json, ui/README.md, examples/knowledge-compiler/README.md
#   KOWALSKI_RELEASE / KOWALSKI_VERSION examples in install.sh and README.md
# Crate READMEs (kowalski, kowalski-core, kowalski-cli, kowalski-mcp-rookery) carry no version
# numbers and no relative links, which crates.io cannot resolve.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CRATES=(kowalski kowalski-core kowalski-cli kowalski-mcp-rookery)
SEMVER='[0-9]+\.[0-9]+\.[0-9]+'

workspace_version() {
  awk '/^\[workspace.package\]/{p=1;next} /^\[/{p=0} p && /^version = /{gsub(/"/,"",$3); print $3; exit}' Cargo.toml
}

# file|perl regex with one capture group holding the version
MIRRORS=(
  "Cargo.toml|^kowalski-core = \\{ path = \"kowalski-core\", version = \"($SEMVER)\""
  "Cargo.toml|^kowalski-cli = \\{ path = \"kowalski-cli\", version = \"($SEMVER)\""
  "ui/package.json|^  \"version\": \"($SEMVER)\""
  "ui/README.md|^\\*\\*Version ($SEMVER)\\*\\*"
  "examples/knowledge-compiler/README.md|release line ($SEMVER)\\*\\*"
  "install.sh|KOWALSKI_RELEASE=v($SEMVER)"
  "install.sh|KOWALSKI_VERSION=($SEMVER)"
  "README.md|KOWALSKI_RELEASE=v($SEMVER)"
  "README.md|KOWALSKI_VERSION=($SEMVER)"
)

check() {
  local want fail=0 entry file re found
  want="$(workspace_version)"
  [[ -n "$want" ]] || { echo "no [workspace.package] version in Cargo.toml" >&2; exit 1; }

  for entry in "${MIRRORS[@]}"; do
    file="${entry%%|*}"
    re="${entry#*|}"
    found="$(perl -ne "print \"\$1\\n\" if /$re/" "$file")"
    if [[ -z "$found" ]]; then
      echo "MISSING  $file: no match for /$re/" >&2
      fail=1
    elif grep -vqx "$want" <<<"$found"; then
      echo "DRIFT    $file: $(tr '\n' ' ' <<<"$found")(want $want)" >&2
      fail=1
    fi
  done

  local c readme
  for c in "${CRATES[@]}"; do
    if ! grep -q '^version.workspace = true' "$c/Cargo.toml"; then
      echo "DRIFT    $c/Cargo.toml: use version.workspace = true" >&2
      fail=1
    fi
    readme="$c/README.md"
    # A version-like x.y.z that is not part of an IP address.
    if perl -ne 'exit 1 if /(?<![\d.])\d+\.\d+\.\d+(?![\d.])/' "$readme"; then :; else
      echo "VERSION  $readme mentions a version; crate READMEs stay version-free:" >&2
      perl -ne 'print "  $.: $_" if /(?<![\d.])\d+\.\d+\.\d+(?![\d.])/' "$readme" >&2
      fail=1
    fi
    if grep -nE '\]\((\.\.?/|[A-Za-z_-]+[./])' "$readme" | grep -vE '\]\((https?://|#|mailto:)' >&2; then
      echo "LINK     $readme has relative links; crates.io needs absolute URLs" >&2
      fail=1
    fi
  done

  if [[ "$fail" -ne 0 ]]; then
    echo "release-version check failed (fix, or run: $0 bump $want)" >&2
    exit 1
  fi
  echo "release-version check ok: $want"
}

bump() {
  local new="$1" entry file re
  [[ "$new" =~ ^$SEMVER$ ]] || { echo "not a version: $new" >&2; exit 1; }
  NEW="$new" perl -0pi -e 's/(\[workspace\.package\]\nversion = ")[^"]+"/${1}$ENV{NEW}"/' Cargo.toml
  for entry in "${MIRRORS[@]}"; do
    file="${entry%%|*}"
    re="${entry#*|}"
    # Replace only the captured version inside each matching line.
    NEW="$new" RE="$re" perl -pi -e 'if (/$ENV{RE}/) { my ($s,$e)=($-[1],$+[1]); substr($_,$s,$e-$s)=$ENV{NEW} }' "$file"
  done
  cargo update --workspace --offline >/dev/null 2>&1 || cargo update --workspace >/dev/null
  echo "bumped to $new (Cargo.lock refreshed); CHANGELOG, ROADMAP and AGENTS.md release lines are prose — edit them by hand"
  check
}

case "${1:-}" in
  check) check ;;
  bump) [[ -n "${2:-}" ]] || { sed -n '5,7p' "$0"; exit 1; }; bump "$2" ;;
  *) sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'; exit 1 ;;
esac
