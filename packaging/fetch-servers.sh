#!/bin/sh
# Download the emperor servers pinned in servers.lock for one target and put their programs
# into a folder, each checked against its pinned checksum.
#
#   packaging/fetch-servers.sh <target> <out-dir>
set -eu
[ $# -eq 2 ] || { sed -n '5p' "$0" | sed 's/^# \{0,1\}//'; exit 1; }
target="$1"; out="$2"
here="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$out"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

sha256_of() { (shasum -a 256 "$1" 2>/dev/null || sha256sum "$1") | awk '{print $1}'; }

found=0
while read -r program version server_target want; do
  case "$program" in '' | '#'*) continue ;; esac
  [ "$server_target" = "$target" ] || continue
  found=1
  asset="$program-$version-$target.tar.gz"
  echo "==> $asset"
  curl -fsSL --retry 3 -o "$tmp/$asset" "https://github.com/yarenty/$program/releases/download/$version/$asset"
  got="$(sha256_of "$tmp/$asset")"
  [ "$got" = "$want" ] || { echo "checksum mismatch for $asset: got $got" >&2; exit 1; }
  mkdir "$tmp/$program"
  tar xzf "$tmp/$asset" -C "$tmp/$program"
  install -m 0755 "$tmp/$program/$program" "$out/$program"
  # A notice that has to travel with the program (emperor-pdf carries fonts).
  for notice in "$tmp/$program"/*NOTICE*; do
    [ -f "$notice" ] && cp "$notice" "$out/$program-$(basename "$notice")"
  done
done < "$here/servers.lock"
[ "$found" = 1 ] || echo "==> no emperor servers are published for $target: this download ships without them"
