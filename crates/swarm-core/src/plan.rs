//! Setup plan: filled by wizard prompts or fully by CLI/env flags (no interaction).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::manifest::RelayRole;
use crate::paths::Paths;

pub const PLAN_SCHEMA: u32 = 1;

/// Complete setup choices. Every field has a CLI flag + env equivalent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupPlan {
    pub schema: u32,
    pub host_username: String,
    pub relay_role: RelayRole,
    /// Preferred relay base URL (stored to host-community/relay.url later).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay_url: Option<String>,
    /// Optional public URL for off-LAN health (env SWARM_PUBLIC_RELAY_URL).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_relay_url: Option<String>,
    /// Override Buzz compose directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compose_dir: Option<String>,
    /// Apply safe auto-fixes during up/setup.
    #[serde(default = "default_true")]
    pub apply_fixes: bool,
    pub updated_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

impl SetupPlan {
    pub fn new(host_username: String, relay_role: RelayRole) -> Self {
        Self {
            schema: PLAN_SCHEMA,
            host_username,
            relay_role,
            relay_url: None,
            public_relay_url: None,
            compose_dir: None,
            apply_fixes: true,
            updated_at: Utc::now(),
        }
    }

    /// Apply env overrides (CLI already merged before save).
    pub fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("SWARM_HOST_USERNAME") {
            if !v.is_empty() {
                self.host_username = v;
            }
        }
        if let Ok(v) = std::env::var("SWARM_RELAY_URL") {
            if !v.is_empty() {
                self.relay_url = Some(v);
            }
        }
        if let Ok(v) = std::env::var("SWARM_PUBLIC_RELAY_URL") {
            if !v.is_empty() {
                self.public_relay_url = Some(v);
            }
        }
        if let Ok(v) = std::env::var("BUZZ_COMPOSE_DIR") {
            if !v.is_empty() {
                self.compose_dir = Some(v);
            }
        }
        if let Ok(v) = std::env::var("SWARM_RELAY_ROLE") {
            if let Some(r) = parse_relay_role(&v) {
                self.relay_role = r;
            }
        }
        if let Ok(v) = std::env::var("SWARM_APPLY_FIXES") {
            self.apply_fixes = matches!(v.to_lowercase().as_str(), "1" | "true" | "yes");
        }
    }

    /// Export values into process env for discover/status.
    pub fn export_env(&self) {
        std::env::set_var("SWARM_HOST_USERNAME", &self.host_username);
        if let Some(ref u) = self.relay_url {
            std::env::set_var("SWARM_RELAY_URL", u);
        }
        if let Some(ref u) = self.public_relay_url {
            std::env::set_var("SWARM_PUBLIC_RELAY_URL", u);
        }
        if let Some(ref d) = self.compose_dir {
            std::env::set_var("BUZZ_COMPOSE_DIR", d);
        }
        let role = match self.relay_role {
            RelayRole::Primary => "primary",
            RelayRole::Standby => "standby",
            RelayRole::Cold => "cold",
            RelayRole::Unknown => "unknown",
        };
        std::env::set_var("SWARM_RELAY_ROLE", role);
    }
}

pub fn parse_relay_role(s: &str) -> Option<RelayRole> {
    match s.trim().to_lowercase().as_str() {
        "primary" | "p" => Some(RelayRole::Primary),
        "standby" | "s" | "secondary" => Some(RelayRole::Standby),
        "cold" | "c" => Some(RelayRole::Cold),
        "unknown" | "u" | "auto" => Some(RelayRole::Unknown),
        _ => None,
    }
}

pub fn load_plan(path: &Path) -> Result<SetupPlan> {
    if !path.exists() {
        return Err(Error::Msg(format!("plan not found at {}", path.display())));
    }
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn save_plan(paths: &Paths, plan: &SetupPlan) -> Result<()> {
    paths.ensure_config_dir()?;
    let mut p = plan.clone();
    p.updated_at = Utc::now();
    let raw = serde_json::to_string_pretty(&p)?;
    fs::write(&paths.plan, raw)?;
    Ok(())
}

pub fn plan_path(paths: &Paths) -> &PathBuf {
    &paths.plan
}

/// Merge partial CLI values over an existing or empty plan.
pub fn merge_plan(
    base: Option<SetupPlan>,
    host_username: Option<String>,
    relay_role: Option<RelayRole>,
    relay_url: Option<String>,
    public_relay_url: Option<String>,
    compose_dir: Option<String>,
    apply_fixes: Option<bool>,
) -> SetupPlan {
    let mut plan = base.unwrap_or_else(|| SetupPlan::new("unknown".into(), RelayRole::Unknown));
    if let Some(v) = host_username {
        if !v.is_empty() {
            plan.host_username = v;
        }
    }
    if let Some(v) = relay_role {
        plan.relay_role = v;
    }
    if let Some(v) = relay_url {
        plan.relay_url = if v.is_empty() { None } else { Some(v) };
    }
    if let Some(v) = public_relay_url {
        plan.public_relay_url = if v.is_empty() { None } else { Some(v) };
    }
    if let Some(v) = compose_dir {
        plan.compose_dir = if v.is_empty() { None } else { Some(v) };
    }
    if let Some(v) = apply_fixes {
        plan.apply_fixes = v;
    }
    plan.apply_env_overrides();
    plan
}

/// True when required fields are present for non-interactive run.
pub fn plan_is_complete(plan: &SetupPlan) -> bool {
    !plan.host_username.is_empty()
        && plan.host_username != "unknown"
        && !matches!(plan.relay_role, RelayRole::Unknown)
}
