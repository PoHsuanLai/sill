//! The dock on a real compositor with fake services, for acceptance (`dev/accept-dock.sh`):
//! one `sill_surfaces::Dock` per output, fed by fixed `watch` channels (COSMIC Files and
//! Terminal pinned, Terminal active with two windows, Firefox running with a count of 7 and
//! 40 % progress), driven by commands on stdin: shell-host's `debug-inject` lines (`sill-dock-<output>
//! hover X Y`, `click X Y`, `leave`, `capture PATH`, `stats`, `quit`) and one of the demo's own,
//! `sill-dock-<output> press primary|secondary|middle X Y`, which presses a button shell-host
//! cannot inject (G10) through the dock's `DockPokes`. Commands the dock sends to the fake
//! services are printed on stderr.
//!
//! `cargo run -p sill-surfaces --example dock_demo`

#[path = "../tests/dock_support/mod.rs"]
mod dock_support;

use std::collections::HashMap;
use std::fmt::Debug;
use std::io::BufRead;
use std::time::Instant;

use shell_host::{Host, HostConfig, HostCtx, HostEvent, OutputId, OutputInfo, Root};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::surfaces::dock::machine::Button;
use sill_surfaces::surfaces::dock::poke::{DockPoke, DockPokes};
use sill_surfaces::surfaces::dock::{DockProps, dock_layer};
use sill_surfaces::{Dock, FrameClock};
use tokio::sync::{mpsc, watch};

/// A line from stdin, or its end.
enum DemoMsg {
    Line(String),
    End,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("dock_demo: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (services, feeds) = dock_support::services();
    let dock_support::DockFeeds {
        bar, pin_events, ..
    } = feeds;
    print_commands("pins", pin_events);
    print_commands("wl", bar.wl);
    print_commands("launch", bar.launch);
    let toplevels = bar.toplevels;
    let settings = SettingsFeed::fixed(ShellFile::default());
    let clock = FrameClock::new(Instant::now());
    let host = Host::<DemoMsg>::connect(HostConfig::default()).map_err(|e| e.to_string())?;
    read_stdin(host.handle());
    let mut docks = HashMap::<OutputId, (String, watch::Sender<Option<DockPoke>>)>::new();
    let mut seq = 0_u64;
    host.run(move |ctx, event| match event {
        HostEvent::Started => {
            // The fake windows live on the first real output.
            if let Some(name) = ctx.outputs().first().and_then(|o| o.name.clone()) {
                toplevels.send_replace(dock_support::toplevels_on(&name));
            }
            for output in ctx.outputs() {
                spawn_dock(ctx, output, &services, &settings, clock, &mut docks);
            }
        }
        HostEvent::OutputAdded(output) => {
            spawn_dock(ctx, output, &services, &settings, clock, &mut docks);
        }
        HostEvent::App(DemoMsg::Line(line)) => {
            seq += 1;
            run_line(ctx, &line, &docks, seq);
        }
        HostEvent::App(DemoMsg::End) => ctx.exit(),
        _ => {}
    })
    .map_err(|e| e.to_string())
}

/// One dock on `output`, with the fake services, the default settings, the demo's clock and a
/// poke channel as root contexts (what the daemon provides, and the harness's presses).
fn spawn_dock(
    ctx: &mut HostCtx<DemoMsg>,
    output: OutputInfo,
    services: &sill_services::Services,
    settings: &SettingsFeed,
    clock: FrameClock,
    docks: &mut HashMap<OutputId, (String, watch::Sender<Option<DockPoke>>)>,
) {
    if docks.contains_key(&output.id) {
        return;
    }
    let config = dock_layer(&output, &ShellFile::default().dock);
    let (pokes, poked) = watch::channel(None);
    docks.insert(output.id, (config.namespace.clone(), pokes));
    let root = Root::with_props(Dock, DockProps { output })
        .context(services.clone())
        .context(settings.clone())
        .context(clock)
        .context(DockPokes(poked))
        .context(sill_surfaces::MEASURE);
    if let Err(error) = ctx.spawn_layer(config, root) {
        eprintln!("dock: {error}");
    }
}

/// A `press` line goes to the named dock's pokes; anything else to shell-host's injection.
fn run_line(
    ctx: &mut HostCtx<DemoMsg>,
    line: &str,
    docks: &HashMap<OutputId, (String, watch::Sender<Option<DockPoke>>)>,
    seq: u64,
) {
    let words: Vec<&str> = line.split_whitespace().collect();
    if let [surface, "press", button, x, y] = words.as_slice() {
        let poke = parse_press(button, x, y, seq);
        let target = docks.values().find(|(name, _)| name == surface);
        match (poke, target) {
            (Some(poke), Some((_, pokes))) => {
                pokes.send_replace(Some(poke));
            }
            _ => eprintln!("dock_demo: {line:?}: no such dock or press"),
        }
        return;
    }
    let applied = shell_host::debug::parse_command(line)
        .map_err(|e| e.to_string())
        .and_then(|command| {
            shell_host::inject::stdin::apply(ctx, command).map_err(|e| e.to_string())
        });
    if let Err(error) = applied {
        eprintln!("debug-inject: {line:?}: {error}");
    }
}

fn parse_press(button: &str, x: &str, y: &str, seq: u64) -> Option<DockPoke> {
    let button = match button {
        "primary" => Button::Primary,
        "secondary" => Button::Secondary,
        "middle" => Button::Middle,
        _ => return None,
    };
    Some(DockPoke {
        seq,
        x: x.parse().ok()?,
        y: y.parse().ok()?,
        button,
    })
}

/// Lines from stdin, on their own thread, into the host's loop.
fn read_stdin(handle: shell_host::HostHandle<DemoMsg>) {
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if handle.send(DemoMsg::Line(line)).is_err() {
                return;
            }
        }
        let _ = handle.send(DemoMsg::End);
    });
}

/// Print every command the dock sends on `rx`, on its own thread.
fn print_commands<C: Debug + Send + 'static>(
    name: &'static str,
    mut rx: mpsc::UnboundedReceiver<C>,
) {
    std::thread::spawn(move || {
        while let Some(command) = rx.blocking_recv() {
            eprintln!("dock_demo: {name} {command:?}");
        }
    });
}
