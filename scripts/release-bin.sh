#!/usr/bin/env bash
# Build stripped macOS release binary and publish to GitHub Releases.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
TAG="${1:-v0.1.0}"
CPU="$(uname -m)"
[[ "$CPU" == "arm64" ]] && CPU="aarch64"
ASSET="swarm-${CPU}-apple-darwin"

echo "→ cargo build --release (max strip)"
export CARGO_PROFILE_RELEASE_STRIP="symbols"
export CARGO_PROFILE_RELEASE_DEBUG="0"
export RUSTFLAGS="-C strip=symbols -C debuginfo=0 -C link-arg=-Wl,-S"
cargo build --release -p swarm-cli
cp "target/release/swarm" "/tmp/${ASSET}"
chmod +x "/tmp/${ASSET}"
# Aggressive strip of local symbols
if command -v strip >/dev/null; then
  strip -xS "/tmp/${ASSET}" 2>/dev/null || strip -x "/tmp/${ASSET}" 2>/dev/null || true
fi
echo "→ $(wc -c </tmp/${ASSET}) bytes"

# Generic leak checks (no fleet-specific constants in this script)
if strings "/tmp/${ASSET}" | grep -Eiq 'privaterelay\.appleid\.com|BEGIN (RSA |OPENSSH )?PRIVATE KEY|CLOUDFLARE_API_TOKEN='; then
  echo "FATAL: binary may contain secrets/private material" >&2
  exit 1
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
