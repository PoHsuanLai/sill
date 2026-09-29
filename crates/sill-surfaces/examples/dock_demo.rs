//! The dock on a real compositor with fake services, for acceptance (`dev/accept-dock.sh`):
//! one `sill_surfaces::Dock` per output, fed by fixed `watch` channels (COSMIC Files and
//! Terminal pinned, Terminal active with two windows, Firefox running with a count of 7 and
//! 40 % progress), driven by shell-host's `debug-inject` commands on stdin (`sill-dock-<output>
//! hover X Y`, `click X Y [primary|secondary|middle]`, `leave`, `capture PATH`, `stats`,
//! `quit`). Commands the dock sends to the fake services are printed on stderr.
//!
//! `cargo run -p sill-surfaces --example dock_demo`

#[path = "../tests/dock_support/mod.rs"]
mod dock_support;

use std::collections::HashSet;
use std::fmt::Debug;
use std::io::BufRead;
use std::time::Instant;

use shell_host::{Host, HostCtx, HostEvent, OutputId, OutputInfo, Root};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::surfaces::dock::{DockProps, dock_layer};
use sill_surfaces::{Dock, FrameClock};
use tokio::sync::mpsc;

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
    let cosmic_wl = bar.cosmic_wl;
    let settings = SettingsFeed::fixed(ShellFile::default());
    let clock = FrameClock::new(Instant::now());
    let host = Host::<DemoMsg>::connect(sill_surfaces::host_config()).map_err(|e| e.to_string())?;
    read_stdin(host.handle());
    let mut docks = HashSet::<OutputId>::new();
    host.run(move |ctx, event| match event {
        HostEvent::Started => {
            // The fake windows live on the first real output.
            if let Some(name) = ctx.outputs().first().and_then(|o| o.name.clone()) {
                cosmic_wl.send_modify(|wl| wl.toplevels = dock_support::toplevels_on(&name));
            }
            for output in ctx.outputs() {
                spawn_dock(ctx, output, &services, &settings, clock, &mut docks);
            }
        }
        HostEvent::OutputAdded(output) => {
            spawn_dock(ctx, output, &services, &settings, clock, &mut docks);
        }
        HostEvent::App(DemoMsg::Line(line)) => run_line(ctx, &line),
        HostEvent::App(DemoMsg::End) => ctx.exit(),
        _ => {}
    })
    .map_err(|e| e.to_string())
}

/// One dock on `output`, with the fake services, the default settings and the demo's clock as
/// root contexts (what the daemon provides).
fn spawn_dock(
    ctx: &mut HostCtx<DemoMsg>,
    output: OutputInfo,
    services: &sill_services::Services,
    settings: &SettingsFeed,
    clock: FrameClock,
    docks: &mut HashSet<OutputId>,
) {
    if !docks.insert(output.id) {
        return;
    }
    let config = dock_layer(&output, &ShellFile::default().dock);
    let root = Root::with_props(Dock, DockProps { output })
        .context(services.clone())
        .context(settings.clone())
        .context(clock)
        .context(sill_surfaces::MEASURE)
        .context(sill_surfaces::FOCUS);
    if let Err(error) = ctx.spawn_layer(config, root) {
        eprintln!("dock: {error}");
    }
}

/// One `debug-inject` line.
fn run_line(ctx: &mut HostCtx<DemoMsg>, line: &str) {
    let applied = shell_host::debug::parse_command(line)
        .map_err(|e| e.to_string())
        .and_then(|command| {
            shell_host::inject::stdin::apply(ctx, command).map_err(|e| e.to_string())
        });
    if let Err(error) = applied {
        eprintln!("debug-inject: {line:?}: {error}");
    }
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
