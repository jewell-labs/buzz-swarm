# Security

## What this project must never contain

- Cloudflare account IDs, tunnel UUIDs, API tokens, or origin certs
- Buzz owner private keys, member secret keys, `.env` compose secrets
- Personal emails, phone numbers, home LAN IPs, or machine-specific hostnames as defaults
- Real production domains hard-coded as “the” fleet URL

## Local state (on your Mac only)

| Path | Contents |
|------|----------|
| `~/.config/buzz-swarm/manifest.json` | Inventory of components this tool manages |
| `~/.config/buzz-swarm/history.jsonl` | Progress log (no secrets by design) |
| `~/.config/host-community/*` | Your host keys (created by you / Buzz; not in git) |
| `~/.cloudflared/*` | Your tunnel credentials (not in git) |

## Reporting

Open a private security advisory on the GitHub repo if you find a secret leak or unsafe default.
