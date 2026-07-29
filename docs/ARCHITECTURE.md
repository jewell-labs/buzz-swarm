# Architecture (buzz-swarm only)

```text
swarm CLI  ──►  swarm-core  ──►  local inventory (docker/brew/launchctl/files)
                    │
                    ▼
         ~/.config/buzz-swarm/manifest.json
```

Buzz relay, Desktop, agents, and MeshLLM are **out of scope** here — see:

- [block/buzz ARCHITECTURE.md](https://github.com/block/buzz/blob/main/ARCHITECTURE.md)
- [buzz-shared-compute-dev.md](https://github.com/block/buzz/blob/main/docs/buzz-shared-compute-dev.md)

## What this layer does

| Step | Ours |
|------|------|
| Discover | Scan local paths/processes for fleet residue |
| Plan | `plan.json` from wizard or flags/env |
| Adopt | Write reverse-able component list |
| Status | Health probes using **your** configured relay URL |
| Uninstall | Reverse owned components (see [UNINSTALL.md](./UNINSTALL.md)) |

## Config sources (ours)

1. Env: `SWARM_RELAY_URL`, `SWARM_PUBLIC_RELAY_URL`, `SWARM_HOST_USERNAME`, `BUZZ_COMPOSE_DIR`
2. Paths under `~/.config/host-community/` if present (created by your Buzz/key setup)
3. Local probe: `http://127.0.0.1:3000/health` only as a local-primary signal
