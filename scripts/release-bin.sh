#!/usr/bin/env bash
# Build stripped macOS release binary and publish to GitHub Releases.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
TAG="${1:-v0.1.0}"
CPU="$(uname -m)"
[[ "$CPU" == "arm64" ]] && CPU="aarch64"
ASSET="swarm-${CPU}-apple-darwin"

echo "→ cargo build --release (strip)"
export RUSTFLAGS="-C strip=symbols -C debuginfo=0"
cargo build --release -p swarm-cli
cp "target/release/swarm" "/tmp/${ASSET}"
chmod +x "/tmp/${ASSET}"
# Extra strip if available
command -v strip >/dev/null && strip -x "/tmp/${ASSET}" 2>/dev/null || true
echo "→ $(wc -c </tmp/${ASSET}) bytes"

# Refuse to ship if personal path / known tunnel UUID leak into binary
if strings "/tmp/${ASSET}" | grep -E '/Users/[^/]+/' | grep -vE 'rustc|cargo/registry|lib/rustlib' | head -5 | grep -q .; then
  echo "warn: binary may contain local path strings (debug metadata)" >&2
fi
if strings "/tmp/${ASSET}" | grep -qE '2f5f3ce5-daef-4ed0-91e6-6183ff0ae150|privaterelay|09f27fe672991450'; then
  echo "FATAL: binary contains forbidden personal/fleet identifiers" >&2
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
