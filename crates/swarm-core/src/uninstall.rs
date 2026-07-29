//! Reverse owned manifest components. Pure planning is unit-tested without I/O.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::manifest::{Component, ComponentKind, Manifest};
use crate::paths::Paths;
use crate::progress::{ProgressEvent, ProgressSink, ProgressStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UninstallMode {
    /// Reverse safe ephemeral components; keep keys, compose volumes, brew, tunnel creds.
    Standard,
    /// Also remove host-community, cloudflared creds, compose volumes, brew if ours.
    Purge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    KillProcess,
    LaunchctlBootout,
    DockerRm,
    DockerComposeDown,
    RmPath,
    BrewUninstall,
    ClearSwarmConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UninstallAction {
    pub kind: ActionKind,
    pub target: String,
    pub component_id: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallReport {
    pub dry_run: bool,
    pub mode: UninstallMode,
    pub actions: Vec<UninstallAction>,
    pub executed: Vec<String>,
    pub errors: Vec<String>,
    pub residuals: Vec<String>,
}

/// Pure: ordered reverse actions from an owned-only view of the manifest.
pub fn plan_uninstall_actions(manifest: &Manifest, mode: UninstallMode) -> Vec<UninstallAction> {
    let owned: Vec<&Component> = manifest.components.iter().filter(|c| c.owned).collect();
    let mut actions = Vec::new();

    // 1) Processes first
    for c in owned.iter().filter(|c| c.kind == ComponentKind::Process) {
        let target = c
            .name
            .clone()
            .unwrap_or_else(|| "host-agent.sh".into());
        actions.push(UninstallAction {
            kind: ActionKind::KillProcess,
            target,
            component_id: c.id.clone(),
            note: "stop agent process".into(),
        });
    }

    // 2) LaunchAgents
    for c in owned.iter().filter(|c| c.kind == ComponentKind::LaunchAgent) {
        let label = c
            .label
            .clone()
            .or_else(|| c.path.as_ref().and_then(|p| Path::new(p).file_stem().map(|s| s.to_string_lossy().into_owned())))
            .unwrap_or_else(|| c.id.clone());
        actions.push(UninstallAction {
            kind: ActionKind::LaunchctlBootout,
            target: label,
            component_id: c.id.clone(),
            note: c.path.clone().unwrap_or_default(),
        });
    }

    // 3) Docker containers (explicit names) then compose projects
    for c in owned.iter().filter(|c| c.kind == ComponentKind::DockerContainer) {
        if let Some(name) = &c.name {
            // openbao always; buzz containers in standard via compose down
            if name.contains("openbao") || mode == UninstallMode::Purge {
                actions.push(UninstallAction {
                    kind: ActionKind::DockerRm,
                    target: name.clone(),
                    component_id: c.id.clone(),
                    note: "docker rm -f".into(),
                });
            }
        }
    }
    // Compose stack: only tear down on Purge (Standard keeps live relay for fleet messaging)
    if mode == UninstallMode::Purge {
        for c in owned.iter().filter(|c| c.kind == ComponentKind::DockerCompose) {
            let path = c.path.clone().unwrap_or_default();
            actions.push(UninstallAction {
                kind: ActionKind::DockerComposeDown,
                target: path,
                component_id: c.id.clone(),
                note: "compose down -v".into(),
            });
        }
    }

    // 4) Files — skip secrets/tunnel unless purge
    for c in owned.iter().filter(|c| c.kind == ComponentKind::Files) {
        let path = match &c.path {
            Some(p) => p.clone(),
            None => continue,
        };
        let is_secrets = c.id.contains("host-community") || path.contains("host-community");
        let is_tunnel_dir = path.contains(".cloudflared");
        if (is_secrets || is_tunnel_dir) && mode != UninstallMode::Purge {
            continue;
        }
        // never delete home root or tracked mesh/repo script trees
        if path == "/" || path == std::env::var("HOME").unwrap_or_default() {
            continue;
        }
        if path.contains("/mesh/scripts") || path.ends_with("/mesh") {
            continue;
        }
        actions.push(UninstallAction {
            kind: ActionKind::RmPath,
            target: path,
            component_id: c.id.clone(),
            note: "rm -rf path".into(),
        });
    }

    // 5) Brew — only purge or explicit installed_by_us with purge
    if mode == UninstallMode::Purge {
        for c in owned.iter().filter(|c| c.kind == ComponentKind::BrewFormula) {
            if c.installed_by_us == Some(true) {
                if let Some(name) = &c.name {
                    actions.push(UninstallAction {
                        kind: ActionKind::BrewUninstall,
                        target: name.clone(),
                        component_id: c.id.clone(),
                        note: "brew uninstall".into(),
                    });
                }
            }
        }
    }

    // CloudflareTunnel components: only purge removes token dir contents via files path
    // (handled if path was .cloudflared under Files; tunnel kind is metadata — skip file ops unless purge and path set)
    if mode == UninstallMode::Purge {
        for c in owned.iter().filter(|c| c.kind == ComponentKind::CloudflareTunnel) {
            if let Some(path) = &c.path {
                actions.push(UninstallAction {
                    kind: ActionKind::RmPath,
                    target: path.clone(),
                    component_id: c.id.clone(),
                    note: "remove cloudflared state dir".into(),
                });
            }
        }
    }

    // 6) Always clear buzz-swarm config last
    actions.push(UninstallAction {
        kind: ActionKind::ClearSwarmConfig,
        target: "buzz-swarm-config".into(),
        component_id: "config.buzz-swarm".into(),
        note: "remove manifest/plan/history".into(),
    });

    actions
}

pub fn execute_uninstall(
    paths: &Paths,
    manifest: &Manifest,
    mode: UninstallMode,
    dry_run: bool,
    sink: &mut dyn ProgressSink,
) -> Result<UninstallReport> {
    let actions = plan_uninstall_actions(manifest, mode);
    let mut report = UninstallReport {
        dry_run,
        mode,
        actions: actions.clone(),
        executed: vec![],
        errors: vec![],
        residuals: vec![],
    };

    let n = actions.len().max(1);
    for (i, action) in actions.iter().enumerate() {
        let pct = ((i as u8 * 90) / n as u8).min(90);
        sink.emit(ProgressEvent::new(
            "uninstall",
            ProgressStatus::Running,
            pct,
            format!("{:?} {}", action.kind, action.target),
        ));
        if dry_run {
            report.executed.push(format!("DRY {:?}", action));
            continue;
        }
        match run_action(paths, action, mode) {
            Ok(msg) => report.executed.push(msg),
            Err(e) => report.errors.push(format!("{}: {e}", action.component_id)),
        }
    }

    if !dry_run {
        // residuals: any owned paths that still exist (secrets kept intentionally)
        for c in manifest.components.iter().filter(|c| c.owned) {
            if let Some(p) = &c.path {
                if Path::new(p).exists() {
                    report
                        .residuals
                        .push(format!("still present (kept or failed): {} ({})", c.id, p));
                }
            }
        }
    }

    sink.emit(ProgressEvent::new(
        "uninstall",
        if report.errors.is_empty() {
            ProgressStatus::Ok
        } else {
            ProgressStatus::Failed
        },
        100,
        format!(
            "done dry_run={} executed={} errors={} residuals={}",
            dry_run,
            report.executed.len(),
            report.errors.len(),
            report.residuals.len()
        ),
    ));

    Ok(report)
}

fn run_action(paths: &Paths, action: &UninstallAction, mode: UninstallMode) -> Result<String> {
    match action.kind {
        ActionKind::KillProcess => {
            let _ = Command::new("pkill")
                .args(["-f", &action.target])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            Ok(format!("pkill -f {}", action.target))
        }
        ActionKind::LaunchctlBootout => {
            let uid = whoami_uid();
            let domain = format!("gui/{uid}");
            let _ = Command::new("launchctl")
                .args(["bootout", &format!("{domain}/{}", action.target)])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            // remove plist if note holds path
            if !action.note.is_empty() && Path::new(&action.note).exists() {
                let _ = fs::remove_file(&action.note);
            } else {
                let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
                let plist = home
                    .join("Library/LaunchAgents")
                    .join(format!("{}.plist", action.target));
                let _ = fs::remove_file(plist);
            }
            Ok(format!("bootout {}", action.target))
        }
        ActionKind::DockerRm => {
            let status = Command::new("docker")
                .args(["rm", "-f", &action.target])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|e| Error::Msg(e.to_string()))?;
            Ok(format!("docker rm -f {} status={}", action.target, status))
        }
        ActionKind::DockerComposeDown => {
            let dir = PathBuf::from(&action.target);
            if !dir.is_dir() {
                return Ok(format!("compose dir missing, skip: {}", action.target));
            }
            let mut args: Vec<String> = vec!["compose".into(), "down".into()];
            if mode == UninstallMode::Purge {
                args.push("-v".into());
            }
            let status = Command::new("docker")
                .args(args.iter().map(|s| s.as_str()))
                .current_dir(&dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|e| Error::Msg(e.to_string()))?;
            Ok(format!("compose down in {} status={}", action.target, status))
        }
        ActionKind::RmPath => {
            let p = PathBuf::from(&action.target);
            if !p.exists() {
                return Ok(format!("already gone: {}", action.target));
            }
            if p.is_dir() {
                fs::remove_dir_all(&p).map_err(Error::Io)?;
            } else {
                fs::remove_file(&p).map_err(Error::Io)?;
            }
            Ok(format!("removed {}", action.target))
        }
        ActionKind::BrewUninstall => {
            let _ = Command::new("brew")
                .args(["uninstall", "--formula", &action.target])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            Ok(format!("brew uninstall {}", action.target))
        }
        ActionKind::ClearSwarmConfig => {
            for f in ["manifest.json", "plan.json", "history.jsonl"] {
                let p = paths.config_dir.join(f);
                let _ = fs::remove_file(p);
            }
            // leave empty dir
            Ok(format!("cleared {}", paths.config_dir.display()))
        }
    }
}

fn whoami_uid() -> String {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "501".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Component, ComponentKind, Manifest, RelayRole};
    use chrono::Utc;

    fn sample_manifest() -> Manifest {
        Manifest {
            schema: 1,
            product: "buzz-swarm".into(),
            host_username: "mac-studio".into(),
            os_user: "ops".into(),
            hostname: "host".into(),
            relay_role: RelayRole::Primary,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            components: vec![
                Component {
                    id: "process.host-agent".into(),
                    kind: ComponentKind::Process,
                    owned: true,
                    path: None,
                    label: None,
                    project: None,
                    name: Some("host-agent.sh".into()),
                    tunnel_id: None,
                    hostnames: vec![],
                    uninstall: None,
                    note: None,
                    installed_by_us: Some(true),
                },
                Component {
                    id: "launchd.com.buzz-swarm.app-sync".into(),
                    kind: ComponentKind::LaunchAgent,
                    owned: true,
                    path: Some("/Users/ops/Library/LaunchAgents/com.buzz-swarm.app-sync.plist".into()),
                    label: Some("com.buzz-swarm.app-sync".into()),
                    project: None,
                    name: None,
                    tunnel_id: None,
                    hostnames: vec![],
                    uninstall: None,
                    note: None,
                    installed_by_us: Some(true),
                },
                Component {
                    id: "docker.buzz-compose".into(),
                    kind: ComponentKind::DockerCompose,
                    owned: true,
                    path: Some("/Users/ops/buzz-ops/compose".into()),
                    label: None,
                    project: Some("buzz".into()),
                    name: None,
                    tunnel_id: None,
                    hostnames: vec![],
                    uninstall: None,
                    note: None,
                    installed_by_us: Some(true),
                },
                Component {
                    id: "files.host-community".into(),
                    kind: ComponentKind::Files,
                    owned: true,
                    path: Some("/Users/ops/.config/host-community".into()),
                    label: None,
                    project: None,
                    name: None,
                    tunnel_id: None,
                    hostnames: vec![],
                    uninstall: None,
                    note: None,
                    installed_by_us: Some(true),
                },
                Component {
                    id: "brew.cloudflared".into(),
                    kind: ComponentKind::BrewFormula,
                    owned: true,
                    path: None,
                    label: None,
                    project: None,
                    name: Some("cloudflared".into()),
                    tunnel_id: None,
                    hostnames: vec![],
                    uninstall: None,
                    note: None,
                    installed_by_us: Some(true),
                },
            ],
            legacy_source: None,
        }
    }

    #[test]
    fn standard_skips_secrets_and_brew() {
        let m = sample_manifest();
        let acts = plan_uninstall_actions(&m, UninstallMode::Standard);
        assert!(acts.iter().any(|a| a.kind == ActionKind::KillProcess));
        assert!(acts.iter().any(|a| a.kind == ActionKind::LaunchctlBootout));
        assert!(!acts.iter().any(|a| a.kind == ActionKind::DockerComposeDown));
        assert!(acts.iter().any(|a| a.kind == ActionKind::ClearSwarmConfig));
        assert!(!acts.iter().any(|a| a.target.contains("host-community")));
        assert!(!acts.iter().any(|a| a.kind == ActionKind::BrewUninstall));
        let kinds: Vec<_> = acts.iter().map(|a| a.kind.clone()).collect();
        let i_proc = kinds.iter().position(|k| *k == ActionKind::KillProcess).unwrap();
        let i_clear = kinds.iter().position(|k| *k == ActionKind::ClearSwarmConfig).unwrap();
        assert!(i_proc < i_clear);
    }

    #[test]
    fn purge_includes_secrets_brew_and_compose() {
        let m = sample_manifest();
        let acts = plan_uninstall_actions(&m, UninstallMode::Purge);
        assert!(acts.iter().any(|a| a.target.contains("host-community")));
        assert!(acts.iter().any(|a| a.kind == ActionKind::BrewUninstall));
        assert!(acts.iter().any(|a| a.kind == ActionKind::DockerComposeDown));
    }

    #[test]
    fn unowned_components_ignored() {
        let mut m = sample_manifest();
        for c in &mut m.components {
            c.owned = false;
        }
        let acts = plan_uninstall_actions(&m, UninstallMode::Standard);
        // only ClearSwarmConfig remains
        assert_eq!(acts.len(), 1);
        assert_eq!(acts[0].kind, ActionKind::ClearSwarmConfig);
    }
}
