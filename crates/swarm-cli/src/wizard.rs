//! Interactive prompts when flags are missing; never used when --yes / --non-interactive.

use std::io::{self, BufRead, Write};

use swarm_core::{
    discover, merge_plan, parse_relay_role, plan_is_complete, Discovery, RelayRole, SetupPlan,
};

/// CLI-visible setup options (flags). All optional so wizard can fill gaps.
#[derive(Debug, Clone, Default)]
pub struct SetupFlags {
    pub host_username: Option<String>,
    pub relay_role: Option<RelayRole>,
    pub relay_url: Option<String>,
    pub public_relay_url: Option<String>,
    pub compose_dir: Option<String>,
    pub apply_fixes: Option<bool>,
    /// Skip all prompts; fail if incomplete.
    pub non_interactive: bool,
    /// Accept defaults for missing optional fields (still errors if required missing in non-interactive).
    pub yes: bool,
}

/// Resolve a complete plan from: flags → saved plan → discovery → prompts (if allowed).
pub fn resolve_plan(
    flags: &SetupFlags,
    saved: Option<SetupPlan>,
    allow_prompt: bool,
) -> Result<SetupPlan, String> {
    let d = discover();
    // Flags win; then saved; then discovery.
    let mut plan = merge_plan(
        saved,
        flags.host_username.clone(),
        flags.relay_role.clone(),
        flags.relay_url.clone(),
        flags.public_relay_url.clone(),
        flags.compose_dir.clone(),
        flags.apply_fixes,
    );

    if plan.host_username.is_empty() || plan.host_username == "unknown" {
        if let Some(ref u) = d.inferred_host_username {
            plan.host_username = u.clone();
        }
    }
    if matches!(plan.relay_role, RelayRole::Unknown) {
        if let Some(r) = infer_role_from_discovery(&d) {
            plan.relay_role = r;
        }
    }
    if plan.relay_url.is_none() {
        plan.relay_url = d.relay_url_configured.clone();
    }
    if plan.public_relay_url.is_none() {
        plan.public_relay_url = d.public_relay_url_configured.clone();
    }

    // Non-interactive path: never prompt
    if flags.non_interactive || flags.yes || !allow_prompt {
        if !plan_is_complete(&plan) {
            return Err(missing_required_msg(&plan));
        }
        return Ok(plan);
    }

    // Interactive wizard (setup, or incomplete up)
    plan = run_wizard(plan, &d)?;
    if !plan_is_complete(&plan) {
        return Err(missing_required_msg(&plan));
    }
    Ok(plan)
}

fn infer_role_from_discovery(d: &Discovery) -> Option<RelayRole> {
    if d.relay_health_local_ok || d.buzz_ops_compose_present {
        Some(RelayRole::Primary)
    } else if d.host_community_present {
        Some(RelayRole::Standby)
    } else {
        None
    }
}

fn missing_required_msg(plan: &SetupPlan) -> String {
    let mut missing = Vec::new();
    if plan.host_username.is_empty() || plan.host_username == "unknown" {
        missing.push("--host-username / SWARM_HOST_USERNAME");
    }
    if matches!(plan.relay_role, RelayRole::Unknown) {
        missing.push("--relay-role primary|standby / SWARM_RELAY_ROLE");
    }
    format!(
        "non-interactive setup incomplete; set: {}",
        missing.join(", ")
    )
}

fn run_wizard(mut plan: SetupPlan, d: &Discovery) -> Result<SetupPlan, String> {
    println!();
    println!("══ buzz-swarm setup ════════════════════════════════");
    println!("  Answer prompts, or re-run with flags for zero prompts:");
    println!("  swarm setup --yes \\");
    println!("    --host-username <name> --relay-role primary|standby \\");
    println!("    [--relay-url URL] [--public-relay-url URL] [--compose-dir PATH]");
    println!();
    println!(
        "  discovered: host={} user={} inferred={:?}",
        d.hostname, d.os_user, d.inferred_host_username
    );
    println!();

    let default_user = if plan.host_username != "unknown" && !plan.host_username.is_empty() {
        plan.host_username.clone()
    } else {
        d.inferred_host_username
            .clone()
            .unwrap_or_else(|| "mac-studio".into())
    };
    plan.host_username = prompt_line(
        "Buzz host username",
        &default_user,
    )?;

    let default_role = match plan.relay_role {
        RelayRole::Primary => "primary",
        RelayRole::Standby => "standby",
        RelayRole::Cold => "cold",
        RelayRole::Unknown => {
            if d.buzz_ops_compose_present || d.relay_health_local_ok {
                "primary"
            } else {
                "standby"
            }
        }
    };
    let role_s = prompt_line(
        "Relay role (primary|standby|cold)",
        default_role,
    )?;
    plan.relay_role = parse_relay_role(&role_s).ok_or_else(|| {
        format!("invalid relay role '{role_s}' (use primary, standby, or cold)")
    })?;

    let default_relay = plan
        .relay_url
        .clone()
        .or_else(|| d.relay_url_configured.clone())
        .unwrap_or_else(|| {
            if matches!(plan.relay_role, RelayRole::Primary) {
                "http://127.0.0.1:3000".into()
            } else {
                String::new()
            }
        });
    let relay = prompt_line(
        "Relay URL (empty to skip)",
        &default_relay,
    )?;
    plan.relay_url = if relay.is_empty() { None } else { Some(relay) };

    let default_pub = plan
        .public_relay_url
        .clone()
        .or_else(|| d.public_relay_url_configured.clone())
        .unwrap_or_default();
    let pub_u = prompt_line(
        "Public relay URL (empty to skip)",
        &default_pub,
    )?;
    plan.public_relay_url = if pub_u.is_empty() { None } else { Some(pub_u) };

    let default_compose = plan
        .compose_dir
        .clone()
        .unwrap_or_else(|| d.paths.buzz_ops_compose.clone());
    let compose = prompt_line(
        "Buzz compose dir (empty = default)",
        &default_compose,
    )?;
    plan.compose_dir = if compose.is_empty() {
        None
    } else {
        Some(compose)
    };

    let fix_default = if plan.apply_fixes { "yes" } else { "no" };
    let fix = prompt_line("Apply safe auto-fixes? (yes/no)", fix_default)?;
    plan.apply_fixes = matches!(fix.to_lowercase().as_str(), "y" | "yes" | "1" | "true");

    println!();
    println!("  plan ready:");
    println!("    host_username = {}", plan.host_username);
    println!("    relay_role    = {:?}", plan.relay_role);
    println!("    relay_url     = {:?}", plan.relay_url);
    println!("    public_url    = {:?}", plan.public_relay_url);
    println!("    compose_dir   = {:?}", plan.compose_dir);
    println!("    apply_fixes   = {}", plan.apply_fixes);
    println!();

    Ok(plan)
}

fn prompt_line(label: &str, default: &str) -> Result<String, String> {
    let mut out = io::stdout();
    if default.is_empty() {
        write!(out, "  {label}: ").map_err(|e| e.to_string())?;
    } else {
        write!(out, "  {label} [{default}]: ").map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let t = line.trim();
    if t.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(t.to_string())
    }
}

/// Stdin is a TTY and we may prompt.
pub fn stdin_is_tty() -> bool {
    use std::io::IsTerminal;
    io::stdin().is_terminal()
}


