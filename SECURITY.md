# Security

## Must never ship in this repo or release binaries

- Cloudflare account IDs, tunnel UUIDs, API tokens, origin certs  
- Buzz owner/member private keys, compose `.env` secrets  
- Personal emails, LAN IPs, or private production domains as defaults  

Buzz secret handling and auth: [block/buzz](https://github.com/block/buzz) (see project security notes / releases).

## Local state (this tool)

| Path | Contents |
|------|----------|
| `~/.config/buzz-swarm/manifest.json` | Inventory of components **we** manage |
| `~/.config/buzz-swarm/history.jsonl` | Progress log (no secret values) |
| `~/.config/buzz-swarm/plan.json` | Setup plan from wizard/flags |

## Reporting

Open a private security advisory on [jewell-labs/buzz-swarm](https://github.com/jewell-labs/buzz-swarm) for leaks or unsafe defaults.
