# AGENTS.md

Humans: [README.md](./README.md).

## Mission

Help any Mac join a **self-hosted Buzz swarm**: inventory what is installed, keep a reverse-able manifest, later install/uninstall with full visibility.

## Non-negotiables

1. **Public-safe code** — no personal identities, fleet tunnel IDs, or private domains as defaults.
2. **Never fork** `block/buzz` product source.
3. **No secrets in git** or release notes.
4. **No OpenBao** (or similar secret sidecars) in this product.
5. Mutations only through `swarm-core` (CLI and future GUI share it).
6. Uninstall only what the **manifest** marks as owned.

## Host identity

Buzz host usernames are **user-chosen** (examples: `mac-studio`, `macbook-pro`, `office-mini`).
Infer from `~/.config/host-community/*.secret_key` basenames or env `SWARM_HOST_USERNAME` — never hard-code a contributor’s OS login.

## Commands

```bash
cargo test -p swarm-core
cargo run -p swarm-cli -- up
```

## Fleet default (locked)

- Relay: **Block free hosted community URL** (not LAN-only, not CF tunnel) for iOS ↔ Mac agents off-LAN.
- Cloudflare: **optional later** (self-host edge / private previews).
- GitHub: **not required** — use community git/artifacts.
- Agent compute stays on each Mac; relay is bus + storage.
