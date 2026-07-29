# Architecture

```text
swarm CLI  ──►  swarm-core  ──►  docker / brew / launchctl / cloudflared / local files
                    │
                    ▼
         ~/.config/buzz-swarm/manifest.json
```

## Phase A (current)

- Discover existing Buzz-related host setup.
- Safe auto-fixes for known residue (e.g. forbidden side containers).
- Write inventory for future install/uninstall.
- One **primary** relay per community; other hosts are **standby**.

## Config sources (in order)

1. Env: `SWARM_RELAY_URL`, `SWARM_PUBLIC_RELAY_URL`, `SWARM_HOST_USERNAME`, `BUZZ_COMPOSE_DIR`
2. `~/.config/host-community/relay.url`
3. Local probes: `http://127.0.0.1:3000/health`
