# AGENTS.md

Humans: [README.md](./README.md).

## Mission

Inventory and reverse multi-Mac **Buzz fleet** hosts via `swarm-core`.  
Buzz product behavior, agents, and shared compute: **[block/buzz](https://github.com/block/buzz)**.

## Non-negotiables

1. Public-safe defaults (no personal IDs, fleet tunnel UUIDs, or private domains baked in).
2. **Never fork** [block/buzz](https://github.com/block/buzz) product source.
3. No secrets in git or release notes.
4. No OpenBao / secret sidecars in this product.
5. Mutations only through `swarm-core`.
6. Uninstall only **owned** manifest components.

## Host identity (fleet)

Buzz member display names / keys are per [Buzz membership](https://github.com/block/buzz/blob/main/README.md).  
This tool infers `SWARM_HOST_USERNAME` from `~/.config/host-community/*.secret_key` basenames or env — never a contributor OS login hardcode.

## Commands

```bash
cargo test -p swarm-core
cargo run -p swarm-cli -- up --yes --host-username <name> --relay-role primary|standby
```

## Fleet default

- Preferred relay URL: hosted community from [buzz.xyz](https://buzz.xyz) (same URL on all devices).
- Self-host compose / tunnel: optional; follow [deploy/compose](https://github.com/block/buzz/tree/main/deploy/compose) and Cloudflare tunnel docs if used.
