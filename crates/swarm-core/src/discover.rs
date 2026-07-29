use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::paths::HostPaths;
use crate::progress::{NullSink, ProgressEvent, ProgressSink, ProgressStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discovery {
    pub hostname: String,
    pub os_user: String,
    pub inferred_host_username: Option<String>,
    pub paths: PathView,
    pub host_community_present: bool,
    pub host_community_keys: Vec<String>,
    pub relay_url_configured: Option<String>,
    pub public_relay_url_configured: Option<String>,
    pub buzz_ops_compose_present: bool,
    pub host_agent_dir_present: bool,
    pub host_agent_running: bool,
    pub docker_available: bool,
    pub docker_containers: Vec<String>,
    pub launch_agents: Vec<String>,
    pub cloudflared_running: bool,
    pub cloudflared_token_present: bool,
    pub cloudflared_cert_present: bool,
    pub cloudflared_brew_installed: bool,
    /// Tunnel id from `cloudflared tunnel list` only — never a baked-in UUID.
    pub tunnel_id_hint: Option<String>,
    pub tunnel_name_hint: Option<String>,
    pub relay_health_local_ok: bool,
    pub relay_health_configured_ok: bool,
    pub relay_health_public_ok: bool,
    pub peers_ssh: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathView {
    pub home: String,
    pub host_community: String,
    pub cloudflared: String,
    pub buzz_ops_compose: String,
    pub host_agent: String,
    pub launch_agents: String,
    pub legacy_apps_root: String,
}

impl From<&HostPaths> for PathView {
    fn from(p: &HostPaths) -> Self {
        Self {
            home: p.home.display().to_string(),
            host_community: p.host_community.display().to_string(),
            cloudflared: p.cloudflared.display().to_string(),
            buzz_ops_compose: p.buzz_ops_compose.display().to_string(),
            host_agent: p.host_agent.display().to_string(),
            launch_agents: p.launch_agents.display().to_string(),
            legacy_apps_root: p.legacy_apps_root.display().to_string(),
        }
    }
}

pub fn discover() -> Discovery {
    let mut sink = NullSink;
    discover_with_progress(HostPaths::default_for_user(), &mut sink)
}

pub fn discover_with_progress(host: HostPaths, sink: &mut dyn ProgressSink) -> Discovery {
    let mut notes = Vec::new();

    emit(sink, "identity", ProgressStatus::Running, 5, "Reading hostname and user…");
    let hostname = run_capture("hostname", &[]).unwrap_or_else(|| "unknown".into());
    let os_user = run_capture("whoami", &[]).unwrap_or_else(|| "unknown".into());
    emit(
        sink,
        "identity",
        ProgressStatus::Ok,
        12,
        format!("host={hostname} user={os_user}"),
    );

    emit(sink, "paths", ProgressStatus::Running, 15, "Scanning host paths…");
    let host_community_keys = list_dir_names(&host.host_community);
    let inferred = infer_host_username(&os_user, &host_community_keys, &hostname);
    let relay_url_configured = read_first_line(&host.host_community.join("relay.url"))
        .or_else(|| std::env::var("SWARM_RELAY_URL").ok());
    let public_relay_url_configured = std::env::var("SWARM_PUBLIC_RELAY_URL").ok().filter(|s| !s.is_empty());

    let buzz_ops_compose_present = host.buzz_ops_compose.join("compose.yml").is_file()
        || host.buzz_ops_compose.join("docker-compose.yml").is_file();
    let host_agent_dir_present = host_agent_dir(&host).is_some();
    emit(
        sink,
        "paths",
        ProgressStatus::Ok,
        25,
        format!(
            "community={} compose={} host_agent={}",
            !host_community_keys.is_empty(),
            buzz_ops_compose_present,
            host_agent_dir_present
        ),
    );

    emit(sink, "launchd", ProgressStatus::Running, 28, "Listing LaunchAgents…");
    let launch_agents = list_swarm_launch_agents(&host.launch_agents);
    emit(
        sink,
        "launchd",
        ProgressStatus::Ok,
        35,
        format!("{} agent label(s)", launch_agents.len()),
    );

    emit(sink, "docker", ProgressStatus::Running, 38, "Querying Docker…");
    let docker_available = command_exists("docker");
    let docker_containers = if docker_available {
        list_docker_containers()
    } else {
        vec![]
    };
    emit(
        sink,
        "docker",
        ProgressStatus::Ok,
        48,
        format!(
            "available={} containers={}",
            docker_available,
            docker_containers.len()
        ),
    );

    emit(sink, "cloudflared", ProgressStatus::Running, 50, "Checking cloudflared…");
    let cloudflared_token_present = dir_has_suffix(&host.cloudflared, ".token");
    let cloudflared_cert_present = host.cloudflared.join("cert.pem").is_file();
    let cloudflared_running = pgrep_contains("cloudflared");
    let cloudflared_brew_installed = brew_list_has("cloudflared");
    let (tunnel_id_hint, tunnel_name_hint) = read_tunnel_hints();
    emit(
        sink,
        "cloudflared",
        ProgressStatus::Ok,
        58,
        format!(
            "running={} token={} brew={}",
            cloudflared_running, cloudflared_token_present, cloudflared_brew_installed
        ),
    );

    emit(
        sink,
        "health",
        ProgressStatus::Running,
        60,
        "Probing relay health…",
    );
    let (relay_health_local_ok, relay_health_configured_ok, relay_health_public_ok) =
        probe_health_parallel(
            relay_url_configured.as_deref(),
            public_relay_url_configured.as_deref(),
        );
    emit(
        sink,
        "health",
        ProgressStatus::Ok,
        78,
        format!(
            "local={} configured={} public={}",
            relay_health_local_ok, relay_health_configured_ok, relay_health_public_ok
        ),
    );

    emit(sink, "peers", ProgressStatus::Running, 80, "SSH peer aliases…");
    let peers_ssh = detect_ssh_peers();
    emit(
        sink,
        "peers",
        ProgressStatus::Ok,
        90,
        format!("{} peer(s)", peers_ssh.len()),
    );

    let host_agent_running = pgrep_host_agent();

    if docker_containers.iter().any(|c| c.contains("openbao")) {
        notes.push("OpenBao container present — not part of buzz-swarm; will be removed on fix".into());
    }

    emit(sink, "discover", ProgressStatus::Ok, 100, "Discovery complete");

    Discovery {
        hostname,
        os_user,
        inferred_host_username: inferred,
        paths: PathView::from(&host),
        host_community_present: host.host_community.is_dir(),
        host_community_keys,
        relay_url_configured,
        public_relay_url_configured,
        buzz_ops_compose_present,
        host_agent_dir_present,
        host_agent_running,
        docker_available,
        docker_containers,
        launch_agents,
        cloudflared_running,
        cloudflared_token_present,
        cloudflared_cert_present,
        cloudflared_brew_installed,
        tunnel_id_hint,
        tunnel_name_hint,
        relay_health_local_ok,
        relay_health_configured_ok,
        relay_health_public_ok,
        peers_ssh,
        notes,
    }
}

fn emit(sink: &mut dyn ProgressSink, step: &str, status: ProgressStatus, pct: u8, msg: impl Into<String>) {
    sink.emit(ProgressEvent::new(step, status, pct, msg));
}

fn probe_health_parallel(
    configured: Option<&str>,
    public: Option<&str>,
) -> (bool, bool, bool) {
    let (tx, rx) = mpsc::channel();

    {
        let tx = tx.clone();
        thread::spawn(move || {
            let ok = http_ok("http://127.0.0.1:3000/health");
            let _ = tx.send(("local", ok));
        });
    }
    {
        let tx = tx.clone();
        let url = configured.map(|s| health_url(s));
        thread::spawn(move || {
            let ok = url.as_ref().map(|u| http_ok(u)).unwrap_or(false);
            let _ = tx.send(("configured", ok));
        });
    }
    {
        let tx = tx.clone();
        let url = public.map(|s| health_url(s));
        thread::spawn(move || {
            let ok = url.as_ref().map(|u| http_ok(u)).unwrap_or(false);
            let _ = tx.send(("public", ok));
        });
    }
    drop(tx);

    let mut local = false;
    let mut configured_ok = false;
    let mut public_ok = false;
    let mut got = 0u8;
    let deadline = std::time::Instant::now() + Duration::from_secs(12);
    while got < 3 && std::time::Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(("local", v)) => {
                local = v;
                got += 1;
            }
            Ok(("configured", v)) => {
                configured_ok = v;
                got += 1;
            }
            Ok(("public", v)) => {
                public_ok = v;
                got += 1;
            }
            Ok(_) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    (local, configured_ok, public_ok)
}

fn health_url(base: &str) -> String {
    let b = base.trim().trim_end_matches('/');
    if b.ends_with("/health") {
        b.to_string()
    } else {
        format!("{b}/health")
    }
}

fn host_agent_dir(host: &HostPaths) -> Option<PathBuf> {
    if host.host_agent.is_dir() {
        return Some(host.host_agent.clone());
    }
    let alt = host.legacy_apps_root.join("scripts/host-agent");
    if alt.is_dir() {
        return Some(alt);
    }
    None
}

fn pgrep_host_agent() -> bool {
    Command::new("pgrep")
        .args(["-fl", "host-agent.sh"])
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Infer Buzz host username from keys / env — never a personal OS login hardcode.
fn infer_host_username(os_user: &str, keys: &[String], hostname: &str) -> Option<String> {
    if let Ok(v) = std::env::var("SWARM_HOST_USERNAME") {
        if !v.is_empty() {
            return Some(v);
        }
    }
    for k in keys {
        if let Some(name) = k.strip_suffix(".secret_key") {
            return Some(name.to_string());
        }
    }
    for k in keys {
        if let Some(name) = k.strip_suffix(".pubkey") {
            return Some(name.to_string());
        }
    }
    // Soft heuristics from hostname only (generic device classes, not people).
    let h = hostname.to_lowercase();
    if h.contains("studio") {
        return Some("mac-studio".into());
    }
    if h.contains("macbook") || h.contains("mba") || h.contains("mbp") {
        return Some("macbook-pro".into());
    }
    if !os_user.is_empty() && os_user != "unknown" {
        // last resort: do not use raw OS user as Buzz identity unless nothing else
        return None;
    }
    None
}

fn list_dir_names(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in rd.flatten() {
        out.push(e.file_name().to_string_lossy().into_owned());
    }
    out.sort();
    out
}

fn list_swarm_launch_agents(dir: &Path) -> Vec<String> {
    list_dir_names(dir)
        .into_iter()
        .filter(|n| n.ends_with(".plist"))
        .filter(|n| {
            n.starts_with("com.buzz-swarm.")
                || n.starts_with("com.jewell.") // legacy label prefix from early experiments
                || n.contains("cloudflared")
        })
        .map(|n| n.trim_end_matches(".plist").to_string())
        .collect()
}

fn list_docker_containers() -> Vec<String> {
    let Some(out) = run_capture("docker", &["ps", "--format", "{{.Names}}"]) else {
        return vec![];
    };
    out.lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn command_exists(bin: &str) -> bool {
    run_capture("which", &[bin]).is_some()
}

fn pgrep_contains(needle: &str) -> bool {
    Command::new("pgrep")
        .args(["-fl", needle])
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

fn brew_list_has(formula: &str) -> bool {
    Command::new("brew")
        .args(["list", "--formula", formula])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn dir_has_suffix(dir: &Path, suffix: &str) -> bool {
    list_dir_names(dir).iter().any(|n| n.ends_with(suffix))
}

/// Only from live `cloudflared tunnel list` — never embed UUIDs in source.
fn read_tunnel_hints() -> (Option<String>, Option<String>) {
    let Some(out) = run_capture("cloudflared", &["tunnel", "list"]) else {
        return (None, None);
    };
    // Skip header; take first data row if present.
    for line in out.lines().skip(1) {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let id = parts[0];
            let name = parts[1];
            if id.len() == 36 && id.contains('-') {
                return (Some(id.to_string()), Some(name.to_string()));
            }
        }
    }
    (None, None)
}

fn http_ok(url: &str) -> bool {
    Command::new("curl")
        .args(["-fsS", "-m", "3", url])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn detect_ssh_peers() -> Vec<String> {
    // Common SSH Host aliases only — not personal IPs.
    let hosts = ["studio", "macbook", "mac-studio", "macbook-pro", "relay"];
    let (tx, rx) = mpsc::channel();
    for host in hosts {
        let tx = tx.clone();
        thread::spawn(move || {
            let ok = Command::new("ssh")
                .args([
                    "-o",
                    "BatchMode=yes",
                    "-o",
                    "ConnectTimeout=1",
                    "-o",
                    "StrictHostKeyChecking=accept-new",
                    host,
                    "true",
                ])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            let _ = tx.send((host, ok));
        });
    }
    drop(tx);
    let mut peers = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok((host, true)) => peers.push(host.to_string()),
            Ok(_) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    while let Ok((host, true)) = rx.try_recv() {
        peers.push(host.to_string());
    }
    peers.sort();
    peers.dedup();
    peers
}

fn read_first_line(path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let line = raw.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

fn run_capture(bin: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(bin).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
