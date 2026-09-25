//! The bar on a real compositor with fake services, for acceptance (`dev/accept-bar-ui.sh`):
//! one `sill_surfaces::Bar` per output, fed by fixed `watch` channels (a Thursday afternoon, a
//! 64 % battery, Wi-Fi joined to `home` with two more networks in range, Bluetooth headphones,
//! the Balanced power mode, a track playing, three workspaces, two terminal windows), driven by
//! shell-host's `debug-inject` commands on stdin (`sill-bar-<output> click X Y`, `stats`,
//! `<surface> capture PATH`, `quit`). Commands the bar sends to the fake services are printed
//! on stderr, never carried out.
//!
//! `cargo run -p sill-surfaces --example bar_demo`

#[path = "../tests/support/fake.rs"]
mod fake;

use std::collections::HashSet;
use std::fmt::Debug;
use std::time::Instant;

use shell_host::{Host, HostConfig, HostCtx, HostEvent, OutputId, OutputInfo, Root};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::surfaces::bar::{BarProps, bar_layer};
use sill_surfaces::{Bar, FrameClock};
use tokio::sync::mpsc;

fn main() {
    if let Err(error) = run() {
        eprintln!("bar_demo: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (services, feeds) = fake::services();
    let fake::Feeds {
        wl,
        volume_commands,
        launch,
        wifi_commands,
        bluetooth_commands,
        profile_commands,
        media_commands,
        media,
        workspaces,
        toplevels,
        ..
    } = feeds;
    print_commands("wl", wl);
    print_commands("volume", volume_commands);
    print_commands("launch", launch);
    // The control center's: printed, never carried out (no real radio or profile changes).
    print_commands("wifi", wifi_commands);
    print_commands("bluetooth", bluetooth_commands);
    print_commands("profile", profile_commands);
    print_commands("media", media_commands);
    // A track playing, for Now Playing (the sender lives as long as the demo).
    media.send_replace(fake::media());
    let settings = SettingsFeed::fixed(ShellFile::default());
    let clock = FrameClock::new(Instant::now());
    let host = Host::<()>::connect(HostConfig::default()).map_err(|e| e.to_string())?;
    // `Started` sees the initial outputs, which may also arrive as `OutputAdded`: one bar each.
    let mut barred = HashSet::<OutputId>::new();
    host.run(move |ctx, event| match event {
        HostEvent::Started => {
            if let Err(error) = shell_host::inject::stdin::install(ctx) {
                eprintln!("debug-inject: {error}");
                ctx.exit();
            }
            // The fake windows and workspaces live on the first real output.
            if let Some(name) = ctx.outputs().first().and_then(|o| o.name.clone()) {
                workspaces.send_replace(fake::workspaces_on(&name));
                toplevels.send_replace(fake::toplevels_on(&name));
            }
            for output in ctx.outputs() {
                if barred.insert(output.id) {
                    spawn_bar(ctx, output, &services, &settings, clock);
                }
            }
        }
        HostEvent::OutputAdded(output) if barred.insert(output.id) => {
            spawn_bar(ctx, output, &services, &settings, clock);
        }
        _ => {}
    })
    .map_err(|e| e.to_string())
}

/// One bar on `output`, with the fake services, the default settings and the demo's clock as
/// root contexts (what the daemon provides).
fn spawn_bar(
    ctx: &mut HostCtx<()>,
    output: OutputInfo,
    services: &sill_services::Services,
    settings: &SettingsFeed,
    clock: FrameClock,
) {
    let config = bar_layer(&output, &ShellFile::default().bar);
    let root = Root::with_props(Bar, BarProps { output })
        .context(services.clone())
        .context(settings.clone())
        .context(clock)
        .context(sill_surfaces::MEASURE);
    if let Err(error) = ctx.spawn_layer(config, root) {
        eprintln!("bar: {error}");
    }
}

/// Print every command the bar sends on `rx`, on its own thread.
fn print_commands<C: Debug + Send + 'static>(
    name: &'static str,
    mut rx: mpsc::UnboundedReceiver<C>,
) {
    std::thread::spawn(move || {
        while let Some(command) = rx.blocking_recv() {
            eprintln!("bar_demo: {name} {command:?}");
        }
    });
}
