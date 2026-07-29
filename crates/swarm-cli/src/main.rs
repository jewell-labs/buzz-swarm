mod ui;

use clap::{Parser, Subcommand};
use swarm_core::paths::HostPaths;
use swarm_core::{
    adopt_from_discovery, apply_safe_fixes, compute_status, discover, discover_with_progress,
    load_manifest, save_manifest, CheckLevel, HistorySink, NullSink, Paths, ProgressEvent,
    ProgressSink, ProgressStatus,
};

#[derive(Parser)]
#[command(
    name = "swarm",
    about = "buzz-swarm — self-hosted Buzz inventory for macOS shared-compute hosts",
    version
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,

    #[arg(long, global = true)]
    json: bool,

    #[arg(long, global = true)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Cmd {
    /// Full auto path: discover → safe fixes → adopt → status
    Up,
    Discover,
    Adopt,
    Status,
    Paths,
}

fn main() {
    let cli = Cli::parse();
    let paths = Paths::default_for_user();
    let _ = paths.ensure_config_dir();

    let code = match cli.cmd {
        Cmd::Up => cmd_up(&paths, cli.json, cli.quiet),
        Cmd::Discover => cmd_discover(&paths, cli.json, cli.quiet),
        Cmd::Adopt => cmd_adopt(&paths, cli.json, cli.quiet),
        Cmd::Status => cmd_status(&paths, cli.json, cli.quiet),
        Cmd::Paths => cmd_paths(&paths, cli.json),
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

fn cmd_up(paths: &Paths, json: bool, quiet: bool) -> i32 {
    let mut sink = make_sink(paths, json, quiet);

    sink.emit(ProgressEvent::new(
        "up",
        ProgressStatus::Running,
        0,
        "buzz-swarm — auto inventory",
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

    sink.emit(ProgressEvent::new(
        "up.fix",
        ProgressStatus::Running,
        55,
        "Applying safe auto-fixes…",
    ));
    let fixes = apply_safe_fixes(&d, sink.as_mut());
    sink.emit(ProgressEvent::new(
        "up.fix",
        ProgressStatus::Ok,
        62,
        if fixes.actions.is_empty() {
            "no fixes needed".into()
        } else {
            format!("{} fix(es)", fixes.actions.len())
        },
    ));

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
    let m = adopt_from_discovery(&d);
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
            serde_json::json!({ "manifest": m, "status": report, "fixes": fixes.actions })
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
            })
        );
    } else {
        println!("config_dir: {}", paths.config_dir.display());
        println!("manifest:   {}", paths.manifest.display());
        println!("history:    {}", paths.history.display());
    }
    0
}
