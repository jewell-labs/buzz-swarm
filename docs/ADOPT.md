# Adopt an existing host

```bash
swarm discover
swarm adopt
swarm status
```

## Paths scanned (generic)

| Path / signal | Meaning |
|---------------|---------|
| `$BUZZ_COMPOSE_DIR` or `~/buzz-ops/compose` | Docker compose for Buzz |
| `~/.config/host-community/` | Host keys, `relay.url` |
| `~/.cloudflared/*.token`, `cert.pem` | Tunnel credentials (presence only) |
| `~/Library/LaunchAgents/com.buzz-swarm.*` | Services this tool owns |
| Docker names containing `buzz` | Relay stack visibility |

Nothing outside your machine is contacted except optional **health URLs you configured**.
