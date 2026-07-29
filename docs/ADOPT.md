# Adopt (buzz-swarm)

Import whatever is already on this Mac into a reverse-able inventory.

```bash
swarm discover
swarm adopt
swarm status
```

Buzz relay install and membership: [block/buzz](https://github.com/block/buzz) / [buzz.xyz](https://buzz.xyz).

## Paths this tool scans (inventory only)

| Path / signal | Why we care |
|---------------|-------------|
| `$BUZZ_COMPOSE_DIR` or `~/buzz-ops/compose` | Optional self-host compose tree |
| `~/.config/host-community/` | Fleet keys + `relay.url` if you keep them here |
| `~/.cloudflared/*` | Tunnel *presence* (optional edge) |
| `~/Library/LaunchAgents/com.buzz-swarm.*` | Services **we** install |
| Docker names containing `buzz` | Visibility for status |

No network calls except health URLs you configured.
