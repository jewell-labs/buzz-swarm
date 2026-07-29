use std::path::PathBuf;

/// On-disk layout for buzz-swarm state (owned by this product).
#[derive(Debug, Clone)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub manifest: PathBuf,
    pub history: PathBuf,
    pub plan: PathBuf,
}

impl Paths {
    pub fn default_for_user() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let config_dir = home.join(".config/buzz-swarm");
        Self {
            manifest: config_dir.join("manifest.json"),
            history: config_dir.join("history.jsonl"),
            plan: config_dir.join("plan.json"),
            config_dir,
        }
    }

    pub fn ensure_config_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config_dir)
    }
}

/// Common locations we may adopt (generic defaults, overridable by env).
#[derive(Debug, Clone)]
pub struct HostPaths {
    pub home: PathBuf,
    pub host_community: PathBuf,
    pub cloudflared: PathBuf,
    pub buzz_ops_compose: PathBuf,
    pub host_agent: PathBuf,
    pub launch_agents: PathBuf,
    /// Optional legacy app tree name used by some early installs.
    pub legacy_apps_root: PathBuf,
}

impl HostPaths {
    pub fn for_home(home: PathBuf) -> Self {
        let compose = std::env::var_os("BUZZ_COMPOSE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("buzz-ops/compose"));
        Self {
            host_community: home.join(".config/host-community"),
            cloudflared: home.join(".cloudflared"),
            buzz_ops_compose: compose,
            host_agent: home.join("host-agent"),
            launch_agents: home.join("Library/LaunchAgents"),
            legacy_apps_root: home.join("mesh"),
            home,
        }
    }

    pub fn default_for_user() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        Self::for_home(home)
    }
}
