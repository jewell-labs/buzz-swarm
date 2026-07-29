# buzz-swarm

Self-hosted **[Buzz](https://github.com/block/buzz)** on macOS for small **shared-compute swarms** (always-on host + laptops).

One CLI: **discover → inventory → (later) install / uninstall**, with a full manifest of everything the tool manages.

**Not** a mesh product. Does **not** fork Buzz server source — only host automation around the official compose/CLI.

## Install (any Mac, no clone)

```bash
curl -fsSL https://raw.githubusercontent.com/jewell-labs/buzz-swarm/main/scripts/install.sh | bash
```

Install only (skip auto inventory):

```bash
curl -fsSL https://raw.githubusercontent.com/jewell-labs/buzz-swarm/main/scripts/install.sh | SWARM_SKIP_UP=1 bash
```

## Commands

```bash
swarm up          # discover → safe fixes → write manifest → status (progress)
swarm discover    # probe only
swarm adopt       # write ~/.config/buzz-swarm/manifest.json
swarm status
swarm paths
```

## What it looks for (generic)

| Area | Examples (discovered, not required) |
|------|-------------------------------------|
| Buzz compose | `~/buzz-ops/compose` or `BUZZ_COMPOSE_DIR` |
| Host identity | `~/.config/host-community/` keys + `relay.url` |
| Tunnel | any `~/.cloudflared/*.token`, running `cloudflared` |
| Services | LaunchAgents matching `com.buzz-swarm.*` (and legacy labels if present) |
| Docker | compose project names containing `buzz` |

Public / LAN health URLs come from **your** config (`relay.url`, env), never from a hardcoded third-party domain.

## Privacy

- No accounts, API tokens, or private keys are shipped in this repo or release binaries.
- The tool only reads local files and process state on the machine where you run it.
- See [SECURITY.md](SECURITY.md).

## Docs

- [Architecture](docs/ARCHITECTURE.md)
- [Adopt existing hosts](docs/ADOPT.md)
- [Uninstall contract](docs/UNINSTALL.md)

## License

MIT
