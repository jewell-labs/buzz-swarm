#!/usr/bin/env bash
# Build stripped macOS release binary and publish to GitHub Releases.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
TAG="${1:-v0.1.0}"
CPU="$(uname -m)"
[[ "$CPU" == "arm64" ]] && CPU="aarch64"
ASSET="swarm-${CPU}-apple-darwin"
HOME_DIR="${HOME}"

echo "→ cargo build --release (strip + path remap)"
export CARGO_PROFILE_RELEASE_STRIP="symbols"
export CARGO_PROFILE_RELEASE_DEBUG="0"
# Remap local absolute paths so release binaries don't embed $HOME
export RUSTFLAGS="-C strip=symbols -C debuginfo=0 --remap-path-prefix=${HOME_DIR}/=/build/ --remap-path-prefix=${ROOT}/=/src/"
cargo build --release -p swarm-cli
cp "target/release/swarm" "/tmp/${ASSET}"
chmod +x "/tmp/${ASSET}"
if command -v strip >/dev/null; then
  strip -xS "/tmp/${ASSET}" 2>/dev/null || strip -x "/tmp/${ASSET}" 2>/dev/null || true
fi
echo "→ $(wc -c </tmp/${ASSET}) bytes"

if strings "/tmp/${ASSET}" | grep -Eiq 'privaterelay\.appleid\.com|BEGIN (RSA |OPENSSH )?PRIVATE KEY|CLOUDFLARE_API_TOKEN='; then
  echo "FATAL: binary may contain secrets/private material" >&2
  exit 1
fi
if strings "/tmp/${ASSET}" | grep -E "/Users/[^/]+/" | grep -vqE '/Users/runner|/Users/host'; then
  # Allow only if no real home usernames
  if strings "/tmp/${ASSET}" | grep -E "/Users/${USER}/" >/dev/null 2>&1; then
    echo "FATAL: binary still embeds builder home path" >&2
    strings "/tmp/${ASSET}" | grep -E "/Users/${USER}/" | head -5 >&2
    exit 1
  fi
fi

if gh release view "$TAG" --repo jewell-labs/buzz-swarm >/dev/null 2>&1; then
  gh release upload "$TAG" "/tmp/${ASSET}" --repo jewell-labs/buzz-swarm --clobber
else
  gh release create "$TAG" "/tmp/${ASSET}" \
    --repo jewell-labs/buzz-swarm \
    --title "$TAG" \
    --notes "buzz-swarm CLI. Install: curl -fsSL https://raw.githubusercontent.com/jewell-labs/buzz-swarm/main/scripts/install.sh | bash"
fi
echo "✓ $TAG / $ASSET"
