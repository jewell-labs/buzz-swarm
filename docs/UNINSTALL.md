# Uninstall contract

`swarm uninstall` (upcoming) reverses the **manifest** only.

## Order (planned)

1. LaunchAgents owned by this tool
2. Cloudflare: this host’s connector/hostnames recorded in the manifest (never bulk-delete a DNS zone)
3. Docker compose `down -v` for owned projects
4. Brew formulas only if marked `installed_by_us`
5. Config dirs this tool created
6. Install root last

Always: `swarm uninstall --dry-run` first.
