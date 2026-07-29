# Uninstall (buzz-swarm)

```bash
swarm uninstall --dry-run
swarm uninstall --yes
swarm uninstall --yes --purge   # also keys, tunnel dir, compose down -v, brew if marked ours
```

## Standard vs purge

| Mode | Removes | Keeps |
|------|---------|--------|
| **Standard** | Processes, LaunchAgents we own, `~/host-agent` copies, `~/.config/buzz-swarm/*` | host-community keys, compose volumes, brew, cloudflared creds |
| **Purge** | Standard + secrets dirs + compose `-v` + brew if `installed_by_us` | Zone registration (never bulk-deleted) |

Never deletes `~/mesh/scripts/**` (repo trees).

Buzz product uninstall / Desktop removal: follow [block/buzz](https://github.com/block/buzz) and macOS app uninstall — not this tool.
