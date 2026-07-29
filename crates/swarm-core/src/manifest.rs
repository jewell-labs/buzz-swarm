use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::discover::Discovery;
use crate::error::{Error, Result};
use crate::paths::Paths;

pub const MANIFEST_SCHEMA: u32 = 1;
pub const PRODUCT: &str = "buzz-swarm";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelayRole {
    Primary,
    Standby,
    Cold,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    DockerCompose,
    DockerContainer,
    LaunchAgent,
    CloudflareTunnel,
    Files,
    BrewFormula,
    Process,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub kind: ComponentKind,
    pub owned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tunnel_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hostnames: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uninstall: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_by_us: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub product: String,
    pub host_username: String,
    pub os_user: String,
    pub hostname: String,
    pub relay_role: RelayRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub components: Vec<Component>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_source: Option<String>,
}

pub fn load_manifest(path: &Path) -> Result<Manifest> {
    if !path.exists() {
        return Err(Error::ManifestMissing(path.display().to_string()));
    }
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn save_manifest(paths: &Paths, manifest: &Manifest) -> Result<()> {
    paths.ensure_config_dir()?;
    let raw = serde_json::to_string_pretty(manifest)?;
    fs::write(&paths.manifest, raw)?;
    Ok(())
}

pub fn adopt_from_discovery(d: &Discovery) -> Manifest {
    let now = Utc::now();
    let mut components = Vec::new();

    if d.buzz_ops_compose_present {
        components.push(Component {
            id: "docker.buzz-compose".into(),
            kind: ComponentKind::DockerCompose,
            owned: true,
            path: Some(d.paths.buzz_ops_compose.clone()),
            label: None,
            project: Some("buzz".into()),
            name: None,
            tunnel_id: None,
            hostnames: vec![],
            uninstall: Some("compose_down_volumes".into()),
            note: Some("Buzz docker compose".into()),
            installed_by_us: Some(true),
        });
    }

    for name in &d.docker_containers {
        if name.contains("openbao") {
            components.push(Component {
                id: format!("docker.container.{name}"),
                kind: ComponentKind::DockerContainer,
                owned: true,
                path: None,
                label: None,
                project: None,
                name: Some(name.clone()),
                tunnel_id: None,
                hostnames: vec![],
                uninstall: Some("docker_rm_volumes".into()),
                note: Some("Not part of product — remove".into()),
                installed_by_us: Some(true),
            });
        } else if name.contains("buzz") {
            components.push(Component {
                id: format!("docker.container.{name}"),
                kind: ComponentKind::DockerContainer,
                owned: true,
                path: None,
                label: None,
                project: Some("buzz".into()),
                name: Some(name.clone()),
                tunnel_id: None,
                hostnames: vec![],
                uninstall: Some("via_compose_project".into()),
                note: None,
                installed_by_us: Some(true),
            });
        }
    }

    for label in &d.launch_agents {
        let owned = label.starts_with("com.buzz-swarm.")
            || label.starts_with("com.jewell.")
            || label.contains("cloudflared");
        components.push(Component {
            id: format!("launchd.{label}"),
            kind: ComponentKind::LaunchAgent,
            owned,
            path: Some(format!("{}/{label}.plist", d.paths.launch_agents)),
            label: Some(label.clone()),
            project: None,
            name: None,
            tunnel_id: None,
            hostnames: vec![],
            uninstall: Some("launchctl_bootout_rm".into()),
            note: None,
            installed_by_us: Some(owned),
        });
    }

    if d.cloudflared_token_present || d.cloudflared_cert_present || d.cloudflared_running {
        components.push(Component {
            id: "cloudflare.tunnel.local".into(),
            kind: ComponentKind::CloudflareTunnel,
            owned: true,
            path: Some(d.paths.cloudflared.clone()),
            label: None,
            project: None,
            name: d.tunnel_name_hint.clone(),
            tunnel_id: d.tunnel_id_hint.clone(),
            hostnames: vec![],
            uninstall: Some("remove_host_connector_and_hostnames".into()),
            note: Some("Zone/DNS account objects are never bulk-deleted by default".into()),
            installed_by_us: Some(true),
        });
    }

    if d.cloudflared_brew_installed {
        components.push(Component {
            id: "brew.cloudflared".into(),
            kind: ComponentKind::BrewFormula,
            owned: true,
            path: None,
            label: None,
            project: None,
            name: Some("cloudflared".into()),
            tunnel_id: None,
            hostnames: vec![],
            uninstall: Some("brew_uninstall_if_ours".into()),
            note: None,
            installed_by_us: Some(true),
        });
    }

    if d.host_community_present {
        components.push(Component {
            id: "files.host-community".into(),
            kind: ComponentKind::Files,
            owned: true,
            path: Some(d.paths.host_community.clone()),
            label: None,
            project: None,
            name: None,
            tunnel_id: None,
            hostnames: vec![],
            uninstall: Some("rm_rf".into()),
            note: None,
            installed_by_us: Some(true),
        });
    }

    if d.host_agent_dir_present {
        let path = {
            let ha = std::path::PathBuf::from(&d.paths.host_agent);
            let alt = std::path::PathBuf::from(&d.paths.legacy_apps_root).join("scripts/host-agent");
            if ha.is_dir() {
                d.paths.host_agent.clone()
            } else if alt.is_dir() {
                alt.display().to_string()
            } else {
                d.paths.host_agent.clone()
            }
        };
        components.push(Component {
            id: "files.host-agent".into(),
            kind: ComponentKind::Files,
            owned: true,
            path: Some(path),
            label: None,
            project: None,
            name: None,
            tunnel_id: None,
            hostnames: vec![],
            uninstall: Some("rm_rf".into()),
            note: None,
            installed_by_us: Some(true),
        });
    }

    if d.host_agent_running {
        components.push(Component {
            id: "process.host-agent".into(),
            kind: ComponentKind::Process,
            owned: true,
            path: None,
            label: None,
            project: None,
            name: Some("host-agent.sh".into()),
            tunnel_id: None,
            hostnames: vec![],
            uninstall: Some("kill_process".into()),
            note: None,
            installed_by_us: Some(true),
        });
    }

    let host_username = d
        .inferred_host_username
        .clone()
        .unwrap_or_else(|| "unknown".into());

    let relay_role = if d.relay_health_local_ok || d.buzz_ops_compose_present {
        RelayRole::Primary
    } else if d.host_community_present {
        RelayRole::Standby
    } else {
        RelayRole::Unknown
    };

    Manifest {
        schema: MANIFEST_SCHEMA,
        product: PRODUCT.into(),
        host_username,
        os_user: d.os_user.clone(),
        hostname: d.hostname.clone(),
        relay_role,
        created_at: now,
        updated_at: now,
        components,
        legacy_source: Some("adopt".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::{Discovery, PathView};

    #[test]
    fn adopt_generic_fixture() {
        let d = Discovery {
            hostname: "relay.local".into(),
            os_user: "ops".into(),
            inferred_host_username: Some("mac-studio".into()),
            paths: PathView {
                home: "/Users/ops".into(),
                host_community: "/Users/ops/.config/host-community".into(),
                cloudflared: "/Users/ops/.cloudflared".into(),
                buzz_ops_compose: "/Users/ops/buzz-ops/compose".into(),
                host_agent: "/Users/ops/host-agent".into(),
                launch_agents: "/Users/ops/Library/LaunchAgents".into(),
                legacy_apps_root: "/Users/ops/mesh".into(),
            },
            host_community_present: true,
            host_community_keys: vec!["mac-studio.secret_key".into()],
            relay_url_configured: Some("http://127.0.0.1:3000".into()),
            public_relay_url_configured: None,
            buzz_ops_compose_present: true,
            host_agent_dir_present: true,
            host_agent_running: false,
            docker_available: true,
            docker_containers: vec!["buzz-prod-relay-1".into()],
            launch_agents: vec!["com.buzz-swarm.app-sync".into()],
            cloudflared_running: true,
            cloudflared_token_present: true,
            cloudflared_cert_present: false,
            cloudflared_brew_installed: true,
            tunnel_id_hint: None,
            tunnel_name_hint: Some("primary".into()),
            relay_health_local_ok: true,
            relay_health_configured_ok: true,
            relay_health_public_ok: false,
            peers_ssh: vec![],
            notes: vec![],
        };
        let m = adopt_from_discovery(&d);
        assert_eq!(m.product, "buzz-swarm");
        assert_eq!(m.host_username, "mac-studio");
        assert!(m.components.iter().any(|c| c.id == "docker.buzz-compose"));
        // Public-safe: no baked-in tunnel UUID in fixture
        assert!(m
            .components
            .iter()
            .filter_map(|c| c.tunnel_id.as_ref())
            .all(|t| t.len() == 36));
    }
}
