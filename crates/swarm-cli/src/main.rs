mod ui;
mod wizard;

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use swarm_core::paths::HostPaths;
use swarm_core::{
    adopt_from_discovery, apply_safe_fixes, compute_status, discover, discover_with_progress,
    load_manifest, load_plan, parse_relay_role, save_manifest, save_plan, CheckLevel, HistorySink,
    NullSink, Paths, ProgressEvent, ProgressSink, ProgressStatus, RelayRole, SetupPlan,
};
use wizard::{resolve_plan, stdin_is_tty, SetupFlags};

#[derive(Parser)]
#[command(
    name = "swarm",
    about = "buzz-swarm — self-hosted Buzz inventory for macOS shared-compute hosts",
    version,
    after_help = "Non-interactive example:\n  swarm setup --yes \\\n    --host-username mac-studio --relay-role primary \\\n    --relay-url http://127.0.0.1:3000\n\n  swarm up --yes --host-username macbook-pro --relay-role standby"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,

    #[arg(long, global = true)]
    json: bool,

    #[arg(long, global = true)]
    quiet: bool,
}

#[derive(Clone, Debug, ValueEnum)]
enum RoleArg {
    Primary,
    Standby,
    Cold,
    Unknown,
}

impl From<RoleArg> for RelayRole {
    fn from(r: RoleArg) -> Self {
        match r {
            RoleArg::Primary => RelayRole::Primary,
            RoleArg::Standby => RelayRole::Standby,
            RoleArg::Cold => RelayRole::Cold,
            RoleArg::Unknown => RelayRole::Unknown,
        }
    }
}

/// Shared setup flags on `setup` and `up` (always available for non-interactive).
#[derive(clap::Args, Debug, Clone)]
struct SetupCli {
    /// Buzz host username (e.g. mac-studio, macbook-pro). Env: SWARM_HOST_USERNAME
    #[arg(long, env = "SWARM_HOST_USERNAME")]
    host_username: Option<String>,

    /// primary | standby | cold. Env: SWARM_RELAY_ROLE
    #[arg(long, env = "SWARM_RELAY_ROLE", value_enum)]
    relay_role: Option<RoleArg>,

    /// Relay base URL. Env: SWARM_RELAY_URL
    #[arg(long, env = "SWARM_RELAY_URL")]
    relay_url: Option<String>,

    /// Public/off-LAN relay URL for health checks. Env: SWARM_PUBLIC_RELAY_URL
    #[arg(long, env = "SWARM_PUBLIC_RELAY_URL")]
    public_relay_url: Option<String>,

    /// Buzz compose directory. Env: BUZZ_COMPOSE_DIR
    #[arg(long, env = "BUZZ_COMPOSE_DIR")]
    compose_dir: Option<PathBuf>,

    /// Apply safe auto-fixes (default true). Env: SWARM_APPLY_FIXES
    #[arg(long, env = "SWARM_APPLY_FIXES", num_args = 0..=1, default_missing_value = "true")]
    apply_fixes: Option<bool>,

    /// Skip prompts; use flags/env/saved plan only (fail if incomplete)
    #[arg(long = "yes", visible_alias = "non-interactive")]
    yes: bool,

    /// Load plan JSON instead of ~/.config/buzz-swarm/plan.json
    #[arg(long)]
    plan: Option<PathBuf>,

    /// Write plan and stop (setup only; up always continues)
    #[arg(long)]
    plan_only: bool,
}

impl SetupCli {
    fn to_flags(&self) -> SetupFlags {
        SetupFlags {
            host_username: self.host_username.clone(),
            relay_role: self.relay_role.clone().map(Into::into),
            relay_url: self.relay_url.clone(),
            public_relay_url: self.public_relay_url.clone(),
            compose_dir: self.compose_dir.as_ref().map(|p| p.display().to_string()),
            apply_fixes: self.apply_fixes,
            non_interactive: self.yes,
            yes: self.yes,
        }
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// Interactive wizard (or fully non-interactive with flags/--yes)
    Setup {
        #[command(flatten)]
        setup: SetupCli,
    },
    /// Discover → fixes → adopt → status (flags for non-interactive)
    Up {
        #[command(flatten)]
        setup: SetupCli,
    },
    Discover,
    Adopt,
    Status,
    Paths,
    /// Print current plan.json
    Plan,
}

fn main() {
    let cli = Cli::parse();
    let paths = Paths::default_for_user();
    let _ = paths.ensure_config_dir();

    let code = match cli.cmd {
        Cmd::Setup { setup } => cmd_setup(&paths, setup, cli.json, cli.quiet),
        Cmd::Up { setup } => cmd_up(&paths, setup, cli.json, cli.quiet),
        Cmd::Discover => cmd_discover(&paths, cli.json, cli.quiet),
        Cmd::Adopt => cmd_adopt(&paths, cli.json, cli.quiet),
        Cmd::Status => cmd_status(&paths, cli.json, cli.quiet),
        Cmd::Paths => cmd_paths(&paths, cli.json),
        Cmd::Plan => cmd_plan(&paths, cli.json),
    };
    std::process::exit(code);
}

fn make_sink(paths: &Paths, json: bool, quiet: bool) -> Box<dyn ProgressSink> {
    let terminal: Box<dyn ProgressSink> = if quiet {
        Box::new(NullSink)
    } else if json {
        Box::new(ui::JsonlSink)
    } else {
        Box::new(ui::TerminalSink::new())
    };
    Box::new(HistorySink::new(paths.history.clone(), Some(terminal)))
}

fn load_saved_plan(paths: &Paths, override_path: &Option<PathBuf>) -> Option<SetupPlan> {
    let path = override_path.as_ref().unwrap_or(&paths.plan);
    load_plan(path).ok()
}

fn build_plan(paths: &Paths, setup: &SetupCli, is_setup_cmd: bool) -> Result<SetupPlan, String> {
    let flags = setup.to_flags();
    let saved = load_saved_plan(paths, &setup.plan);
    // Prompts only when: TTY, not --yes/--non-interactive.
    // setup: wizard preferred (even if discovery filled defaults).
    // up: wizard only when plan still incomplete.
    let allow_prompt = stdin_is_tty() && !flags.non_interactive && !setup.yes;
    let mut plan = resolve_plan(&flags, saved, allow_prompt && !is_setup_cmd)?;
    if is_setup_cmd && allow_prompt {
        // Always offer wizard on `setup` unless non-interactive
        plan = resolve_plan(&flags, Some(plan), true)?;
    }
    Ok(plan)
}

fn cmd_setup(paths: &Paths, setup: SetupCli, json: bool, quiet: bool) -> i32 {
    let force_wizard = !setup.yes;
    let plan = match build_plan(paths, &setup, force_wizard) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("setup: {e}");
            return 2;
        }
    };
    if let Err(e) = save_plan(paths, &plan) {
        eprintln!("setup: save plan: {e}");
        return 1;
    }
    plan.export_env();

    if !quiet && !json {
        println!("✓ plan saved → {}", paths.plan.display());
        print_plan_human(&plan);
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&plan).unwrap());
    }
    if setup.plan_only {
        return 0;
    }
    // Continue into inventory path
    run_inventory(paths, &plan, json, quiet)
}

fn cmd_up(paths: &Paths, setup: SetupCli, json: bool, quiet: bool) -> i32 {
    // Prefer non-interactive for `up` when flags cover it; prompt only if incomplete + TTY
    let plan = match build_plan(paths, &setup, false) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("up: {e}");
            eprintln!("hint: swarm setup   # interactive");
            eprintln!("      swarm up --yes --host-username NAME --relay-role primary|standby");
            return 2;
        }
    };
    let _ = save_plan(paths, &plan);
    plan.export_env();
    if setup.plan_only {
        if json {
            println!("{}", serde_json::to_string_pretty(&plan).unwrap());
        }
        return 0;
    }
    run_inventory(paths, &plan, json, quiet)
}

fn run_inventory(paths: &Paths, plan: &SetupPlan, json: bool, quiet: bool) -> i32 {
    plan.export_env();
    let mut sink = make_sink(paths, json, quiet);

    sink.emit(ProgressEvent::new(
        "up",
        ProgressStatus::Running,
        0,
        format!("buzz-swarm — host={}", plan.host_username),
    ));

    sink.emit(ProgressEvent::new(
        "up.discover",
        ProgressStatus::Running,
        5,
        "Discovering this host…",
    ));
    let d = discover_with_progress(HostPaths::default_for_user(), sink.as_mut());
    sink.emit(ProgressEvent::new(
        "up.discover",
        ProgressStatus::Ok,
        50,
        format!("username={:?}", d.inferred_host_username),
    ));

    let fixes = if plan.apply_fixes {
        sink.emit(ProgressEvent::new(
            "up.fix",
            ProgressStatus::Running,
            55,
            "Applying safe auto-fixes…",
        ));
        let f = apply_safe_fixes(&d, sink.as_mut());
        sink.emit(ProgressEvent::new(
            "up.fix",
            ProgressStatus::Ok,
            62,
            if f.actions.is_empty() {
                "no fixes needed".into()
            } else {
                format!("{} fix(es)", f.actions.len())
            },
        ));
        f
    } else {
        sink.emit(ProgressEvent::new(
            "up.fix",
            ProgressStatus::Skipped,
            62,
            "fixes disabled",
        ));
        swarm_core::FixReport { actions: vec![] }
    };

    sink.emit(ProgressEvent::new(
        "up.rediscover",
        ProgressStatus::Running,
        65,
        "Re-scanning…",
    ));
    let d = discover_with_progress(HostPaths::default_for_user(), sink.as_mut());
    sink.emit(ProgressEvent::new(
        "up.rediscover",
        ProgressStatus::Ok,
        75,
        "scan complete",
    ));

    sink.emit(ProgressEvent::new(
        "up.adopt",
        ProgressStatus::Running,
        78,
        "Writing manifest…",
    ));
    let mut m = adopt_from_discovery(&d);
    // Prefer explicit plan identity
    if plan.host_username != "unknown" {
        m.host_username = plan.host_username.clone();
    }
    if !matches!(plan.relay_role, RelayRole::Unknown) {
        m.relay_role = plan.relay_role.clone();
    }
    if let Err(e) = save_manifest(paths, &m) {
        sink.emit(ProgressEvent::new(
            "up.adopt",
            ProgressStatus::Failed,
            78,
            e.to_string(),
        ));
        return 1;
    }
    sink.emit(ProgressEvent::new(
        "up.adopt",
        ProgressStatus::Ok,
        88,
        format!("{} components", m.components.len()),
    ));

    let report = compute_status(&d, Some(&m));
    sink.emit(ProgressEvent::new(
        "up.status",
        ProgressStatus::Ok,
        97,
        format!("overall={:?}", report.overall),
    ));
    sink.emit(ProgressEvent::new(
        "up",
        if matches!(report.overall, CheckLevel::Fail) {
            ProgressStatus::Failed
        } else {
            ProgressStatus::Ok
        },
        100,
        format!(
            "done — host={} role={:?} owned={}/{}",
            m.host_username, m.relay_role, report.owned_count, report.component_count
        ),
    ));

    if !json && !quiet {
        println!();
        ui::print_status_human(&report);
        println!();
        ui::print_next_steps(&report, &m);
    } else if json {
        println!(
            "{}",
            serde_json::json!({
                "plan": plan,
                "manifest": m,
                "status": report,
                "fixes": fixes.actions,
            })
        );
    }

    match report.overall {
        CheckLevel::Fail => 2,
        _ => 0,
    }
}

fn cmd_discover(paths: &Paths, json: bool, quiet: bool) -> i32 {
    let mut sink = make_sink(paths, json, quiet);
    let d = discover_with_progress(HostPaths::default_for_user(), sink.as_mut());
    if json {
        println!("{}", serde_json::to_string_pretty(&d).unwrap());
    } else if !quiet {
        ui::print_discovery_human(&d);
    }
    0
}

fn cmd_adopt(paths: &Paths, json: bool, quiet: bool) -> i32 {
    let mut sink = make_sink(paths, json, quiet);
    let d = discover_with_progress(HostPaths::default_for_user(), sink.as_mut());
    let m = adopt_from_discovery(&d);
    if let Err(e) = save_manifest(paths, &m) {
        eprintln!("{e}");
        return 1;
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&m).unwrap());
    } else if !quiet {
        ui::print_manifest_human(&m, paths);
    }
    0
}

fn cmd_status(paths: &Paths, json: bool, quiet: bool) -> i32 {
    let d = discover();
    let manifest = load_manifest(&paths.manifest).ok();
    let report = compute_status(&d, manifest.as_ref());
    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else if !quiet {
        ui::print_status_human(&report);
    }
    match report.overall {
        CheckLevel::Fail => 2,
        CheckLevel::Warn => 1,
        _ => 0,
    }
}

fn cmd_paths(paths: &Paths, json: bool) -> i32 {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "config_dir": paths.config_dir,
                "manifest": paths.manifest,
                "history": paths.history,
                "plan": paths.plan,
            })
        );
    } else {
        println!("config_dir: {}", paths.config_dir.display());
        println!("manifest:   {}", paths.manifest.display());
        println!("history:    {}", paths.history.display());
        println!("plan:       {}", paths.plan.display());
    }
    0
}

fn cmd_plan(paths: &Paths, json: bool) -> i32 {
    match load_plan(&paths.plan) {
        Ok(p) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&p).unwrap());
            } else {
                print_plan_human(&p);
            }
            0
        }
        Err(e) => {
            eprintln!("no plan yet ({e}); run: swarm setup");
            1
        }
    }
}

fn print_plan_human(p: &SetupPlan) {
    println!("══ Plan ═══════════════════════════════════════════");
    println!("  host_username = {}", p.host_username);
    println!("  relay_role    = {:?}", p.relay_role);
    println!("  relay_url     = {:?}", p.relay_url);
    println!("  public_url    = {:?}", p.public_relay_url);
    println!("  compose_dir   = {:?}", p.compose_dir);
    println!("  apply_fixes   = {}", p.apply_fixes);
    println!("  updated_at    = {}", p.updated_at);
}

// silence unused import if parse_relay_role only used via RoleArg
#[allow(dead_code)]
fn _parse_role(s: &str) -> Option<RelayRole> {
    parse_relay_role(s)
}
