#!/usr/bin/env bash
# Kowalski one-line installer (crates.io).
#
#   curl -fsSL https://raw.githubusercontent.com/yarenty/kowalski/main/install.sh | bash
#
# Optional custom domain (redirect or mirror this file):
#   curl -fsSL https://yarenty.com/kowalski/install.sh | bash
#
# Environment:
#   KOWALSKI_VERSION=1.3.0     Pin crates.io version (published line; 1.4.0+ from git until 1.5 publish)
#   KOWALSKI_FEATURES=postgres   Enable postgres feature on kowalski + kowalski-cli
#   KOWALSKI_INSTALL_MCP=1       Also install kowalski-mcp-rookery (+ datafusion; slow)
#   KOWALSKI_SKIP_RUSTUP=1       Do not auto-install Rust when missing
#
# Installs into $HOME/.cargo/bin — ensure that directory is on your PATH.
set -euo pipefail

KOWALSKI_REPO="${KOWALSKI_REPO:-https://github.com/yarenty/kowalski}"
KOWALSKI_VERSION="${KOWALSKI_VERSION:-}"
KOWALSKI_FEATURES="${KOWALSKI_FEATURES:-}"
KOWALSKI_INSTALL_MCP="${KOWALSKI_INSTALL_MCP:-0}"

info() { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

usage() {
  sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
}

[[ "${1:-}" == "-h" || "${1:-}" == "--help" ]] && usage

ensure_cargo() {
  if command -v cargo >/dev/null 2>&1; then
    # shellcheck disable=SC1091
    [[ -f "${HOME}/.cargo/env" ]] && source "${HOME}/.cargo/env"
    return 0
  fi

  [[ "${KOWALSKI_SKIP_RUSTUP:-0}" == "1" ]] && die "cargo not found; install Rust from https://rustup.rs or unset KOWALSKI_SKIP_RUSTUP"

  info "Rust not found — installing via rustup (non-interactive)"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
  command -v cargo >/dev/null 2>&1 || die "cargo still not on PATH after rustup"
}

cargo_install() {
  local crate="$1"
  shift
  local -a args=(install "$crate")
  [[ -n "$KOWALSKI_VERSION" ]] && args+=(--version "$KOWALSKI_VERSION")
  while [[ $# -gt 0 ]]; do
    args+=("$1")
    shift
  done
  info "cargo ${args[*]}"
  cargo "${args[@]}"
}

ensure_path_hint() {
  local bin="${HOME}/.cargo/bin"
  if ! command -v kowalski-cli >/dev/null 2>&1; then
    warn "${bin} may not be on PATH — add: export PATH=\"${bin}:\$PATH\""
  fi
}

seed_config() {
  local dest_dir="${KOWALSKI_CONFIG_DIR:-${HOME}/.config/kowalski}"
  local dest="${dest_dir}/config.toml"
  if [[ -f "$dest" ]]; then
    info "config already exists: ${dest}"
    return 0
  fi
  mkdir -p "$dest_dir"
  local sample_url="${KOWALSKI_REPO}/raw/main/config.toml"
  if curl -fsSL "$sample_url" -o "$dest"; then
    info "wrote sample config: ${dest}"
  else
    warn "could not download sample config from ${sample_url}"
  fi
}

check_ollama() {
  if command -v ollama >/dev/null 2>&1; then
    info "ollama found: $(command -v ollama)"
    return 0
  fi
  warn "ollama not found — default config uses local Ollama (https://ollama.com)"
  warn "or set [llm] provider = \"openai\" in your config.toml"
}

main() {
  info "Kowalski installer (crates.io)"
  ensure_cargo

  local -a feat
  feat=()
  if [[ -n "$KOWALSKI_FEATURES" ]]; then
    feat=(--features "$KOWALSKI_FEATURES")
  fi

  cargo_install kowalski-cli "${feat[@]}"
  cargo_install kowalski "${feat[@]}"

  if [[ "$KOWALSKI_INSTALL_MCP" == "1" ]]; then
    info "Installing optional MCP servers (DataFusion compile is slow)"
    cargo_install kowalski-mcp-rookery
    cargo_install kowalski-mcp-datafusion
  fi

  ensure_path_hint
  local cfg_dir="${KOWALSKI_CONFIG_DIR:-${HOME}/.config/kowalski}"
  local cfg_file="${cfg_dir}/config.toml"
  seed_config
  check_ollama

  cat <<EOF

Kowalski installed.

  export PATH="\${HOME}/.cargo/bin:\${PATH}"
  kowalski-cli doctor
  kowalski-cli config check "${cfg_file}"
  kowalski -c "${cfg_file}"    # HTTP API (default 127.0.0.1:3456)

Config: ${cfg_file}
Docs:   ${KOWALSKI_REPO}

UI (optional): clone the repo and run \`cd ui && bun install && bun run dev\`
EOF
}

main "$@"
