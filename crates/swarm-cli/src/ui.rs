use std::io::{self, Write};
use swarm_core::{
    CheckLevel, Discovery, Manifest, Paths, ProgressEvent, ProgressSink, ProgressStatus,
    RelayRole, StatusReport,
};

pub struct TerminalSink;

impl TerminalSink {
    pub fn new() -> Self {
        Self
    }
}

impl ProgressSink for TerminalSink {
    fn emit(&mut self, event: ProgressEvent) {
        let icon = match event.status {
            ProgressStatus::Pending => "·",
            ProgressStatus::Running => "…",
            ProgressStatus::Ok => "✓",
            ProgressStatus::Failed => "✗",
            ProgressStatus::Skipped => "–",
        };
        let bar = progress_bar(event.pct, 20);
        let line = format!(
            "  {icon} [{bar}] {pct:>3}%  {step:<16} {msg}",
            pct = event.pct,
            step = event.step,
            msg = event.msg
        );
        let mut out = io::stdout();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

pub struct JsonlSink;
impl ProgressSink for JsonlSink {
    fn emit(&mut self, event: ProgressEvent) {
        let _ = writeln!(io::stderr(), "{}", event.to_json_line());
        let _ = io::stderr().flush();
    }
}

fn progress_bar(pct: u8, width: usize) -> String {
    let filled = (pct as usize * width) / 100;
    let mut s = String::with_capacity(width);
    for i in 0..width {
        s.push(if i < filled { '█' } else { '░' });
    }
    s
}

pub fn print_discovery_human(d: &Discovery) {
    println!();
    println!("══ Discovery ══════════════════════════════════════");
    println!("  host:     {} ({})", d.hostname, d.os_user);
    println!("  username: {:?}", d.inferred_host_username);
    println!(
        "  keys:     {} ({} files)",
        yn(d.host_community_present),
        d.host_community_keys.len()
    );
    println!("  compose:  {}", yn(d.buzz_ops_compose_present));
    println!(
        "  docker:   {}  {} container(s)",
        yn(d.docker_available),
        d.docker_containers.len()
    );
    println!("  launchd:  {} label(s)", d.launch_agents.len());
    println!(
        "  tunnel:   running={} token={}",
        d.cloudflared_running, d.cloudflared_token_present
    );
    println!(
        "  health:   local={} configured={} public={}",
        d.relay_health_local_ok, d.relay_health_configured_ok, d.relay_health_public_ok
    );
    println!("  peers:    {:?}", d.peers_ssh);
    for n in &d.notes {
        println!("  note:     {n}");
    }
    println!();
}

pub fn print_manifest_human(m: &Manifest, paths: &Paths) {
    println!();
    println!("══ Manifest ═══════════════════════════════════════");
    println!("  path:     {}", paths.manifest.display());
    println!("  host:     {}  role={:?}", m.host_username, m.relay_role);
    println!("  count:    {}", m.components.len());
    for c in &m.components {
        println!("  • {:<40} owned={}", c.id, c.owned);
    }
    println!();
}

pub fn print_status_human(r: &StatusReport) {
    println!("══ Status ═════════════════════════════════════════");
    println!(
        "  overall:  {}  host={:?}  role={:?}",
        level_badge(&r.overall),
        r.host_username,
        r.relay_role
    );
    println!(
        "  inventory: {}/{} owned",
        r.owned_count, r.component_count
    );
    for c in &r.checks {
        println!(
            "  {}  {:<18} {}",
            level_badge(&c.level),
            c.id,
            c.message
        );
    }
}

pub fn print_next_steps(r: &StatusReport, m: &Manifest) {
    println!("══ Next ═══════════════════════════════════════════");
    let mut steps = Vec::new();
    if r.checks.iter().any(|c| c.id == "openbao") {
        steps.push("OpenBao still present — re-run swarm up");
    }
    if matches!(m.relay_role, RelayRole::Primary)
        && !r
            .checks
            .iter()
            .any(|c| c.id == "relay.local" && matches!(c.level, CheckLevel::Ok))
    {
        steps.push("Primary role but local relay unhealthy — check Docker compose");
    }
    if steps.is_empty() {
        steps.push("Inventory complete. Full install/uninstall engines come next.");
    }
    for (i, s) in steps.iter().enumerate() {
        println!("  {}. {s}", i + 1);
    }
    println!();
}

fn level_badge(l: &CheckLevel) -> &'static str {
    match l {
        CheckLevel::Ok => "OK  ",
        CheckLevel::Warn => "WARN",
        CheckLevel::Fail => "FAIL",
        CheckLevel::Info => "info",
    }
}

fn yn(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}
