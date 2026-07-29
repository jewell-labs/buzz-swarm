#!/usr/bin/env bash
# buzz-swarm — one-liner install (no clone).
#
#   curl -fsSL https://raw.githubusercontent.com/jewell-labs/buzz-swarm/main/scripts/install.sh | bash
#
# Env: SWARM_VERSION, SWARM_BIN_DIR, SWARM_SKIP_UP, SWARM_FORCE_CARGO
set -euo pipefail

REPO="jewell-labs/buzz-swarm"
BIN_DIR="${SWARM_BIN_DIR:-$HOME/.local/bin}"
VERSION="${SWARM_VERSION:-}"
ARCH="$(uname -m)"
OS="$(uname -s)"

log() { printf '→ %s\n' "$*"; }
ok()  { printf '✓ %s\n' "$*"; }
die() { printf '✗ %s\n' "$*" >&2; exit 1; }

[[ "$OS" == "Darwin" ]] || die "macOS only (got $OS)"

mkdir -p "$BIN_DIR"
case ":$PATH:" in *":$BIN_DIR:"*) ;; *) export PATH="$BIN_DIR:$PATH" ;; esac

latest_tag() {
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | python3 -c 'import sys,json; print(json.load(sys.stdin).get("tag_name",""))' 2>/dev/null || true
}

asset_name() {
  local cpu="$ARCH"
  [[ "$cpu" == "arm64" ]] && cpu="aarch64"
  echo "swarm-${cpu}-apple-darwin"
}

install_from_release() {
  local tag="$1" asset url tmp
  asset="$(asset_name)"
  url="https://github.com/${REPO}/releases/download/${tag}/${asset}"
  log "Downloading ${asset} (${tag})…"
  tmp="$(mktemp)"
  curl -fsSL "$url" -o "$tmp" || { rm -f "$tmp"; return 1; }
  chmod +x "$tmp"
  mv "$tmp" "$BIN_DIR/swarm"
  ok "Installed $BIN_DIR/swarm"
}

install_via_cargo() {
  log "Building via cargo from git…"
  if ! command -v cargo >/dev/null 2>&1; then
    log "Installing rustup (minimal)…"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
  fi
  cargo install --git "https://github.com/${REPO}.git" --locked --bin swarm --root "$HOME/.local" --force
  ok "cargo install → $BIN_DIR/swarm"
}

log "buzz-swarm installer"
if [[ -z "${SWARM_FORCE_CARGO:-}" ]]; then
  [[ -z "$VERSION" ]] && VERSION="$(latest_tag)"
  if [[ -n "$VERSION" && "$VERSION" != "null" ]] && install_from_release "$VERSION"; then
    :
  else
    install_via_cargo
  fi
else
  install_via_cargo
fi

command -v swarm >/dev/null || die "swarm not on PATH (add $BIN_DIR)"
ok "swarm $(swarm --version 2>/dev/null || echo ok)"

if [[ -n "${SWARM_SKIP_UP:-}" ]]; then
  log "SWARM_SKIP_UP set — done"
  exit 0
fi

log "Running: swarm up"
echo
swarm up
