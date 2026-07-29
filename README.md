# buzz-swarm

macOS host inventory + install/uninstall for a **multi-Mac Buzz fleet**.

Buzz product install, Desktop, CLI, relay, shared compute, and agents are **not documented here** — use Block’s SSOT:

| Topic | Official docs |
|-------|----------------|
| Product / install | [block/buzz](https://github.com/block/buzz) · [README](https://github.com/block/buzz/blob/main/README.md) |
| Hosted community | [buzz.xyz](https://buzz.xyz) (create/join community; use that URL as relay) |
| Self-host compose | [block/buzz `deploy/compose`](https://github.com/block/buzz/tree/main/deploy/compose) |
| Shared compute / local models | [docs/buzz-shared-compute-dev.md](https://github.com/block/buzz/blob/main/docs/buzz-shared-compute-dev.md) |
| CLI (`buzz …`) | ship with Buzz Desktop / releases — `buzz --help` |
| Architecture | [ARCHITECTURE.md](https://github.com/block/buzz/blob/main/ARCHITECTURE.md) |

This repo **does not fork** Buzz. It only automates **fleet host inventory** around whatever Buzz setup you already run.

---

## What buzz-swarm adds

1. **`swarm` CLI** — discover, plan, adopt, status, uninstall with a reverse-able **manifest**
2. **Non-interactive setup** — every field is a flag **and** env var (`--yes` never prompts)
3. **Multi-host inventory** — primary vs standby roles, path scanning, optional tunnel *detection*
4. **Safe reverse** — `swarm uninstall` clears what the manifest owns (standard keeps keys/compose volumes)

It does **not** replace Buzz Desktop, compose bootstrap, model download, or community membership.

---

## Install (this tool only)

```bash
curl -fsSL https://raw.githubusercontent.com/jewell-labs/buzz-swarm/main/scripts/install.sh | bash
```

Skip auto inventory:

```bash
curl -fsSL https://raw.githubusercontent.com/jewell-labs/buzz-swarm/main/scripts/install.sh | SWARM_SKIP_UP=1 bash
```

## Commands

```bash
swarm setup           # wizard, or flags + --yes
swarm up              # discover → fixes → adopt → status
swarm plan
swarm discover | adopt | status | paths
swarm uninstall --dry-run
swarm uninstall --yes
```

### Non-interactive

```bash
swarm setup --yes \
  --host-username mac-studio \
  --relay-role primary \
  --relay-url https://YOUR.communities.buzz.xyz
```

| Flag | Env |
|------|-----|
| `--host-username` | `SWARM_HOST_USERNAME` |
| `--relay-role` | `SWARM_RELAY_ROLE` |
| `--relay-url` | `SWARM_RELAY_URL` |
| `--public-relay-url` | `SWARM_PUBLIC_RELAY_URL` |
| `--compose-dir` | `BUZZ_COMPOSE_DIR` |
| `--yes` | — |

State: `~/.config/buzz-swarm/{manifest,plan,history}.jsonl?`  
See [docs/ADOPT.md](docs/ADOPT.md), [docs/UNINSTALL.md](docs/UNINSTALL.md), [SECURITY.md](SECURITY.md).

## Fleet note (our default)

For iOS off-LAN + Mac agents: use a **hosted community URL** from [buzz.xyz](https://buzz.xyz) as the same `SWARM_RELAY_URL` / Buzz client relay on every device. Cloudflare Tunnel is **optional** and only relevant if you self-host a public edge — see Cloudflare’s tunnel docs, not this README.

## License

MIT
