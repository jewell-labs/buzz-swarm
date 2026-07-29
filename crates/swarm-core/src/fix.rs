//! Safe auto-fixes (no secrets, no data-bearing deletes beyond forbidden residue).

use std::process::Command;

use crate::discover::Discovery;
use crate::progress::{ProgressEvent, ProgressSink, ProgressStatus};

#[derive(Debug, Clone)]
pub struct FixReport {
    pub actions: Vec<String>,
}

pub fn apply_safe_fixes(d: &Discovery, sink: &mut dyn ProgressSink) -> FixReport {
    let mut actions = Vec::new();

    // Deprecated early-experiment LaunchAgent labels (duplicate of app-sync).
    let has_app_sync = d
        .launch_agents
        .iter()
        .any(|l| l.ends_with(".app-sync") || l == "com.jewell.app-sync" || l == "com.buzz-swarm.app-sync");
    let mesh_sync = d
        .launch_agents
        .iter()
        .find(|l| l.contains("mesh-sync"))
        .cloned();
    if has_app_sync {
        if let Some(label) = mesh_sync {
            sink.emit(ProgressEvent::new(
                "fix.mesh-sync",
                ProgressStatus::Running,
                0,
                format!("Removing deprecated {label}…"),
            ));
            let uid = run_capture("id", &["-u"]).unwrap_or_else(|| "501".into());
            let domain = format!("gui/{uid}");
            let _ = Command::new("launchctl")
                .args(["bootout", &format!("{domain}/{label}")])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            let plist = format!("{}/{label}.plist", d.paths.launch_agents);
            let _ = std::fs::remove_file(&plist);
            actions.push(format!("removed LaunchAgent {label}"));
            sink.emit(ProgressEvent::new(
                "fix.mesh-sync",
                ProgressStatus::Ok,
                100,
                "deprecated agent removed",
            ));
        }
    }

    let openbao: Vec<_> = d
        .docker_containers
        .iter()
        .filter(|c| c.contains("openbao"))
        .cloned()
        .collect();
    if !openbao.is_empty() {
        sink.emit(ProgressEvent::new(
            "fix.openbao",
            ProgressStatus::Running,
            0,
            "Removing unsupported OpenBao container…",
        ));
        for c in &openbao {
            let _ = Command::new("docker")
                .args(["rm", "-f", c])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
        actions.push("removed OpenBao container(s)".into());
        sink.emit(ProgressEvent::new(
            "fix.openbao",
            ProgressStatus::Ok,
            100,
            "OpenBao removed",
        ));
    }

    if actions.is_empty() {
        sink.emit(ProgressEvent::new(
            "fix",
            ProgressStatus::Skipped,
            100,
            "No safe auto-fixes needed",
        ));
    }

    FixReport { actions }
}

fn run_capture(bin: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(bin).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
