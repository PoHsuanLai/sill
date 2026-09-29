//! The dock on a real compositor with fake services, for acceptance (`dev/accept/dock.sh`):
//! one `sill_ui_kit::Dock` per output, fed by fixed `watch` channels (COSMIC Files and
//! Terminal pinned, Terminal active with two windows, Firefox running with a count of 7 and
//! 40 % progress), driven by shell-host's `debug-inject` commands on stdin (`sill-dock-<output>
//! hover X Y`, `click X Y [primary|secondary|middle]`, `leave`, `capture PATH`, `stats`,
//! `quit`). Commands the dock sends to the fake services are printed on stderr.
//!
//! `cargo run -p sill-ui-kit --example dock_demo`

use std::collections::HashSet;
use std::fmt::Debug;
use std::io::BufRead;
use std::time::Instant;

use shell_host::{Host, HostCtx, HostEvent, HostRoot, OutputId, OutputInfo};
use sill_settings::ShellFile;
use sill_ui_kit::Dock;
use sill_ui_kit::clock::FrameClock;
use sill_ui_kit::harness::fake;
use sill_ui_kit::settings::SettingsFeed;
use sill_ui_kit::surface::{MountProps, Surface, mount};
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
    let (services, feeds) = fake::services();
    feeds.apps.send_replace(fake::dock::index());
    feeds.pins.send_replace(fake::dock::pins());
    feeds.badges.send_replace(fake::dock::badges());
    print_commands("pins", feeds.pin_commands);
    print_commands("wl", feeds.wl);
    print_commands("launch", feeds.launch);
    let cosmic_wl = feeds.cosmic_wl;
    let settings = SettingsFeed::fixed(ShellFile::default());
    let clock = FrameClock::new(Instant::now());
    let host =
        Host::<DemoMsg>::connect(sill_ui_kit::root::host_config()).map_err(|e| e.to_string())?;
    read_stdin(host.handle());
    let mut docks = HashSet::<OutputId>::new();
    host.run(move |ctx, event| match event {
        HostEvent::Started => {
            // The fake windows live on the first real output.
            if let Some(name) = ctx.outputs().first().and_then(|o| o.name.clone()) {
                cosmic_wl.send_modify(|wl| wl.toplevels = fake::dock::toplevels_on(&name));
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
    services: &sill_ui_kit::services::Services,
    settings: &SettingsFeed,
    clock: FrameClock,
    docks: &mut HashSet<OutputId>,
) {
    if !docks.insert(output.id) {
        return;
    }
    let config = Dock::layer(&output, &ShellFile::default(), ());
    let root = HostRoot::with_props(mount::<Dock>, MountProps::<Dock>::new(output, ()))
        .context(services.clone())
        .context(settings.clone())
        .context(clock);
    if let Err(error) = ctx.spawn_layer(config, root) {
        eprintln!("dock: {error}");
    }
}

/// One `debug-inject` line.
fn run_line(ctx: &mut HostCtx<DemoMsg>, line: &str) {
    let applied = shell_host::inject::parse_command(line)
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
