#!/usr/bin/env bash
# Kowalski one-line installer.
#
#   curl -fsSL https://raw.githubusercontent.com/yarenty/kowalski/main/install.sh | bash
#
# Downloads the pre-built binaries for your machine (macOS or Linux, Intel or ARM) from the latest
# GitHub release into ~/.local/bin, then `kowalski` opens the app with its Setup screen. No Rust
# needed. Other platforms, or KOWALSKI_FROM_SOURCE=1, build from crates.io instead.
#
# Environment:
#   KOWALSKI_BIN_DIR=~/.local/bin  Where the binaries go (download mode)
#   KOWALSKI_RELEASE=v2.6.1        A specific release tag instead of the latest
#   KOWALSKI_FROM_SOURCE=1         Build with cargo from crates.io (no UI inside)
#   KOWALSKI_VERSION=2.6.1         crates.io version (source mode; default: latest)
#   KOWALSKI_FEATURES=postgres     Features for kowalski + kowalski-cli (source mode)
#   KOWALSKI_INSTALL_MCP=1         Also install kowalski-mcp-rookery (source mode)
#   KOWALSKI_SKIP_RUSTUP=1         Do not auto-install Rust when missing (source mode)
#   KOWALSKI_NO_MODIFY_PATH=1      Do not add the bin dir to your shell profile (download mode)
set -euo pipefail

KOWALSKI_REPO="${KOWALSKI_REPO:-https://github.com/yarenty/kowalski}"
KOWALSKI_VERSION="${KOWALSKI_VERSION:-}"
KOWALSKI_FEATURES="${KOWALSKI_FEATURES:-}"
KOWALSKI_INSTALL_MCP="${KOWALSKI_INSTALL_MCP:-0}"
KOWALSKI_BIN_DIR="${KOWALSKI_BIN_DIR:-${HOME}/.local/bin}"
KOWALSKI_RELEASE="${KOWALSKI_RELEASE:-}"
KOWALSKI_FROM_SOURCE="${KOWALSKI_FROM_SOURCE:-0}"

info() { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

usage() {
  sed -n '2,21p' "$0" | sed 's/^# \{0,1\}//'
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
  local sample_url="${KOWALSKI_REPO}/raw/main/config.example.toml"
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
  warn "ollama not found — install it for a free local model (https://ollama.com),"
  warn "or pick a hosted model with your own key on the Setup screen"
}

# Release asset target for this machine, or nothing when no pre-built binary exists.
release_target() {
  case "$(uname -s)/$(uname -m)" in
    Darwin/arm64) echo "aarch64-apple-darwin" ;;
    Darwin/x86_64) echo "x86_64-apple-darwin" ;;
    Linux/x86_64) echo "x86_64-unknown-linux-gnu" ;;
    Linux/aarch64 | Linux/arm64) echo "aarch64-unknown-linux-gnu" ;;
    *) echo "" ;;
  esac
}

# Put the bin dir on PATH for new terminals (the way rustup and bun do): one line in the profile
# of the user's shell. The current terminal cannot be changed from here, so the closing message
# shows the full path to run now.
add_to_path() {
  local dir="$1" line profile
  line="export PATH=\"${dir}:\$PATH\""
  if [[ "${KOWALSKI_NO_MODIFY_PATH:-0}" == "1" ]]; then
    warn "${dir} is not on PATH — add: ${line}"
    return 0
  fi
  case "$(basename "${SHELL:-}")" in
    zsh) profile="${ZDOTDIR:-$HOME}/.zshrc" ;;
    bash) [[ "$(uname -s)" == "Darwin" ]] && profile="${HOME}/.bash_profile" || profile="${HOME}/.bashrc" ;;
    *) profile="${HOME}/.profile" ;;
  esac
  if [[ -f "$profile" ]] && grep -qF "$line" "$profile"; then
    return 0
  fi
  printf '\n# Added by the kowalski installer\n%s\n' "$line" >> "$profile"
  info "added ${dir} to PATH in ${profile} (new terminals pick it up)"
}

install_binary() {
  local target="$1"
  local asset="kowalski-${target}.tar.gz"
  local base="${KOWALSKI_REPO}/releases/latest/download"
  [[ -n "$KOWALSKI_RELEASE" ]] && base="${KOWALSKI_REPO}/releases/download/${KOWALSKI_RELEASE}"
  local tmp
  tmp="$(mktemp -d)"
  info "downloading ${asset}"
  if ! curl -fsSL "${base}/${asset}" -o "${tmp}/${asset}"; then
    rm -rf "$tmp"
    return 1
  fi
  if curl -fsSL "${base}/${asset}.sha256" -o "${tmp}/${asset}.sha256" 2>/dev/null; then
    local want got
    want="$(awk '{print $1}' "${tmp}/${asset}.sha256")"
    got="$( (shasum -a 256 "${tmp}/${asset}" 2>/dev/null || sha256sum "${tmp}/${asset}") | awk '{print $1}')"
    [[ "$want" == "$got" ]] || die "checksum mismatch for ${asset}"
  fi
  tar xzf "${tmp}/${asset}" -C "$tmp"
  mkdir -p "$KOWALSKI_BIN_DIR"
  install -m 0755 "${tmp}/kowalski-${target}/kowalski" "${tmp}/kowalski-${target}/kowalski-cli" "$KOWALSKI_BIN_DIR/"
  rm -rf "$tmp"
  info "installed kowalski and kowalski-cli into ${KOWALSKI_BIN_DIR}"
  local run="kowalski"
  case ":${PATH}:" in
    *":${KOWALSKI_BIN_DIR}:"*) ;;
    *)
      add_to_path "$KOWALSKI_BIN_DIR"
      run="${KOWALSKI_BIN_DIR}/kowalski"
      ;;
  esac
  check_ollama
  printf '\nKowalski installed.\n\n  %s        # opens the app in your browser; first start asks three setup questions\n\n' "$run"
  printf 'Model: local Ollama (free, https://ollama.com) or any OpenAI-compatible endpoint with your key.\n'
  printf 'Docs:  %s\n' "$KOWALSKI_REPO"
}

main() {
  local target
  target="$(release_target)"
  if [[ "$KOWALSKI_FROM_SOURCE" != "1" && -n "$target" ]]; then
    install_binary "$target" && return 0
    warn "no pre-built binary downloaded; building from source instead"
  fi
  info "Kowalski installer (crates.io)"
  ensure_cargo

  local -a feat
  feat=()
  if [[ -n "$KOWALSKI_FEATURES" ]]; then
    feat=(--features "$KOWALSKI_FEATURES")
  fi

  cargo_install kowalski-cli ${feat[@]+"${feat[@]}"}
  cargo_install kowalski ${feat[@]+"${feat[@]}"}

  if [[ "$KOWALSKI_INSTALL_MCP" == "1" ]]; then
    info "Installing optional MCP servers"
    cargo_install kowalski-mcp-rookery
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
  kowalski                     # server on http://127.0.0.1:3456/ (finds ${cfg_file} by itself)

Config: ${cfg_file}
Docs:   ${KOWALSKI_REPO}

UI: a crates.io build carries no UI. For the full app in one binary, build it from a clone:
  git clone ${KOWALSKI_REPO} && cd kowalski
  (cd ui && bun install && bun run build) && cargo install --path kowalski
Then \`kowalski\` opens the UI in your browser.
EOF
}

main "$@"
