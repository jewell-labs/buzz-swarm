use serde::{Deserialize, Serialize};

use crate::discover::Discovery;
use crate::manifest::{ComponentKind, Manifest, RelayRole};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckLevel {
    Ok,
    Warn,
    Fail,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub id: String,
    pub level: CheckLevel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReport {
    pub overall: CheckLevel,
    pub host_username: Option<String>,
    pub relay_role: Option<RelayRole>,
    pub checks: Vec<Check>,
    pub component_count: usize,
    pub owned_count: usize,
}

pub fn compute_status(discovery: &Discovery, manifest: Option<&Manifest>) -> StatusReport {
    let mut checks = Vec::new();

    checks.push(Check {
        id: "host.identity".into(),
        level: CheckLevel::Info,
        message: format!(
            "hostname={} os_user={} buzz_username={:?}",
            discovery.hostname, discovery.os_user, discovery.inferred_host_username
        ),
    });

    if discovery.relay_health_local_ok {
        checks.push(Check {
            id: "relay.local".into(),
            level: CheckLevel::Ok,
            message: "http://127.0.0.1:3000/health ok".into(),
        });
    } else if discovery.relay_health_configured_ok {
        checks.push(Check {
            id: "relay.configured".into(),
            level: CheckLevel::Ok,
            message: format!(
                "configured relay ok ({})",
                discovery
                    .relay_url_configured
                    .as_deref()
                    .unwrap_or("relay.url")
            ),
        });
    } else {
        checks.push(Check {
            id: "relay".into(),
            level: CheckLevel::Warn,
            message: "no local/configured relay health (set relay.url or run a primary)".into(),
        });
    }

    if discovery.public_relay_url_configured.is_some() {
        if discovery.relay_health_public_ok {
            checks.push(Check {
                id: "relay.public".into(),
                level: CheckLevel::Ok,
                message: "SWARM_PUBLIC_RELAY_URL health ok".into(),
            });
        } else {
            checks.push(Check {
                id: "relay.public".into(),
                level: CheckLevel::Warn,
                message: "SWARM_PUBLIC_RELAY_URL set but health failed".into(),
            });
        }
    }

    if discovery.cloudflared_running {
        checks.push(Check {
            id: "tunnel".into(),
            level: CheckLevel::Ok,
            message: "cloudflared process running".into(),
        });
    } else if discovery.cloudflared_token_present || discovery.cloudflared_cert_present {
        let level = if discovery.relay_health_local_ok || discovery.buzz_ops_compose_present {
            CheckLevel::Warn
        } else {
            CheckLevel::Info
        };
        checks.push(Check {
            id: "tunnel".into(),
            level,
            message: "cloudflared credentials present; connector not running on this host".into(),
        });
    }

    if discovery.docker_containers.iter().any(|c| c.contains("openbao")) {
        checks.push(Check {
            id: "openbao".into(),
            level: CheckLevel::Fail,
            message: "OpenBao container present — not supported; run swarm up to remove".into(),
        });
    }

    if discovery.host_community_present {
        checks.push(Check {
            id: "identity.keys".into(),
            level: CheckLevel::Ok,
            message: format!(
                "host-community present ({} file(s))",
                discovery.host_community_keys.len()
            ),
        });
    } else {
        checks.push(Check {
            id: "identity.keys".into(),
            level: CheckLevel::Warn,
            message: "no ~/.config/host-community yet".into(),
        });
    }

    for n in &discovery.notes {
        checks.push(Check {
            id: "note".into(),
            level: CheckLevel::Info,
            message: n.clone(),
        });
    }

    let (component_count, owned_count, host_username, relay_role) = if let Some(m) = manifest {
        checks.push(Check {
            id: "manifest".into(),
            level: CheckLevel::Ok,
            message: format!(
                "manifest loaded: {} components ({} owned)",
                m.components.len(),
                m.components.iter().filter(|c| c.owned).count()
            ),
        });
        let tunnels = m
            .components
            .iter()
            .filter(|c| c.kind == ComponentKind::CloudflareTunnel)
            .count();
        if tunnels > 0 {
            checks.push(Check {
                id: "manifest.tunnel".into(),
                level: CheckLevel::Info,
                message: format!("{tunnels} tunnel component(s) tracked"),
            });
        }
        (
            m.components.len(),
            m.components.iter().filter(|c| c.owned).count(),
            Some(m.host_username.clone()),
            Some(m.relay_role.clone()),
        )
    } else {
        checks.push(Check {
            id: "manifest".into(),
            level: CheckLevel::Warn,
            message: "no manifest yet — run: swarm adopt".into(),
        });
        (0, 0, discovery.inferred_host_username.clone(), None)
    };

    StatusReport {
        overall: overall_level(&checks),
        host_username,
        relay_role,
        checks,
        component_count,
        owned_count,
    }
}

fn overall_level(checks: &[Check]) -> CheckLevel {
    if checks.iter().any(|c| matches!(c.level, CheckLevel::Fail)) {
        CheckLevel::Fail
    } else if checks.iter().any(|c| matches!(c.level, CheckLevel::Warn)) {
        CheckLevel::Warn
    } else {
        CheckLevel::Ok
    }
}
