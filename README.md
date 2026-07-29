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

## Setup: interactive **or** fully non-interactive

### Interactive wizard

```bash
swarm setup
```

Prompts for host username, relay role, URLs, compose dir, fixes. Saves `~/.config/buzz-swarm/plan.json`, then runs inventory.

### Non-interactive (CI / automation)

Every wizard field has a flag **and** env var. Use `--yes` (alias `--non-interactive`) to never prompt; missing **required** fields error out.

```bash
swarm setup --yes \
  --host-username mac-studio \
  --relay-role primary \
  --relay-url http://127.0.0.1:3000 \
  --public-relay-url https://buzz.example.com \
  --compose-dir "$HOME/buzz-ops/compose"

# inventory only (same flags)
swarm up --yes --host-username macbook-pro --relay-role standby
```

| Flag | Env | Required |
|------|-----|----------|
| `--host-username` | `SWARM_HOST_USERNAME` | yes |
| `--relay-role` `primary\|standby\|cold` | `SWARM_RELAY_ROLE` | yes |
| `--relay-url` | `SWARM_RELAY_URL` | no |
| `--public-relay-url` | `SWARM_PUBLIC_RELAY_URL` | no |
| `--compose-dir` | `BUZZ_COMPOSE_DIR` | no |
| `--apply-fixes` / omit | `SWARM_APPLY_FIXES` | no (default true) |
| `--yes` | — | skips all prompts |
| `--plan-only` | — | write plan.json only |
| `--plan PATH` | — | load plan from file |

## Commands

```bash
swarm setup           # wizard (or flags + --yes)
swarm up              # discover → fixes → adopt → status
swarm plan            # show saved plan
swarm discover
swarm adopt
swarm status
swarm paths
```

## What it looks for (generic)

| Area | Examples (discovered, not required) |
|------|-------------------------------------|
| Buzz compose | `~/buzz-ops/compose` or `BUZZ_COMPOSE_DIR` |
| Host identity | `~/.config/host-community/` keys + `relay.url` |
| Tunnel | any `~/.cloudflared/*.token`, running `cloudflared` |
| Services | LaunchAgents matching `com.buzz-swarm.*` |
| Docker | names containing `buzz` |

Public / LAN health URLs come from **your** config (`relay.url`, env), never a hardcoded third-party domain.

## Recommended path (2–4 weeks)

**Use Block’s free hosted community** as the relay so **iOS (cellular) can talk to Mac agents anytime**.

```text
iOS  ──https──►  <name>.communities.buzz.xyz  ◄──  mac-studio / macbook-pro agents
```

| Need | Free hosted community | Cloudflare Tunnel |
|------|----------------------|-------------------|
| iOS ↔ agents off-LAN | Yes (both dial out to Block) | Only if you self-host the relay |
| Agents + git/artifacts, no GitHub | Yes (on community) | Optional edge later |
| Interactive app previews on your domain | Not a product path yet | Self-host + edge later |

**Cloudflare is optional** — inventory may detect a tunnel; messaging does **not** require one.

Non-interactive example (hosted community):

```bash
swarm setup --yes \
  --host-username mac-studio \
  --relay-role standby \
  --relay-url https://YOUR.communities.buzz.xyz
```

(Use the same `--relay-url` on every host and in the iOS app.)

## Privacy

See [SECURITY.md](SECURITY.md). No secrets in the repo or release binaries.

## License

MIT
