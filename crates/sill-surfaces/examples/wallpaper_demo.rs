//! The wallpaper on a real compositor with the *real* wallpaper service (everything else fake),
//! for acceptance (`dev/accept-wallpaper.sh`): one `sill_surfaces::Wallpaper` per output,
//! prepared against whatever COSMIC background config the environment has — the acceptance
//! script writes one pointing at a marked test image before running this, so the real
//! `sill_services::wallpaper::spawn` resizes and caches a real file the surface then shows.
//! Driven by shell-host's `debug-inject` commands on stdin (`stats`, `<surface> capture PATH`,
//! `quit`; `sill-bar-demo`'s own convention, `bar_demo.rs`'s module doc).
//!
//! `cargo run -p sill-surfaces --example wallpaper_demo`

#[path = "../tests/support/fake.rs"]
mod fake;

use std::collections::HashSet;

use shell_host::{Host, HostCtx, HostEvent, HostRoot, OutputId, OutputInfo};
use shell_host::{Size, buffer_size};
use sill_model::ids::OutputName;
use sill_model::wallpaper::WallpaperCommand;
use sill_services::Control;
use sill_surfaces::Wallpaper;
use sill_surfaces::services::Services;
use sill_surfaces::surfaces::wallpaper::{WallpaperProps, wallpaper_layer};

fn main() {
    if let Err(error) = run() {
        eprintln!("wallpaper_demo: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    // The wallpaper service's own task needs an entered multi-thread runtime for its whole
    // life, independent of whatever shell-host's `Host` enters for its own event loop (the same
    // split the real daemon makes: `crates/sill/src/daemon.rs::run` builds its own runtime for
    // the daemon's `start` before `Host::connect`). Kept alive by staying in scope for the rest of
    // this function, which does not return until `host.run` does.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let (mut services, _feeds) = fake::services();
    let wallpaper = sill_services::service::spawn::<sill_services::WallpaperService>(
        runtime.handle(),
        &sill_services::env::Env::from_process(),
        (),
    );
    let wallpaper_control = wallpaper.control.clone();
    services.wallpaper.state = wallpaper.state;
    services.wallpaper.control = wallpaper.control;

    let host =
        Host::<()>::connect(sill_surfaces::root::host_config()).map_err(|e| e.to_string())?;
    let mut done = HashSet::<OutputId>::new();
    host.run(move |ctx, event| match event {
        HostEvent::Started => {
            if let Err(error) = shell_host::inject::stdin::install(ctx) {
                eprintln!("debug-inject: {error}");
                ctx.exit();
            }
            for output in ctx.outputs() {
                if done.insert(output.id) {
                    spawn_wallpaper(ctx, output, &services, &wallpaper_control);
                }
            }
        }
        HostEvent::OutputAdded(output) if done.insert(output.id) => {
            spawn_wallpaper(ctx, output, &services, &wallpaper_control);
        }
        _ => {}
    })
    .map_err(|e| e.to_string())
}

/// One wallpaper on `output`: a `Prepare` at its buffer size (the resize needs real pixels, not
/// logical ones) alongside the `Ds` surface that shows the result once the service publishes
/// it.
fn spawn_wallpaper(
    ctx: &mut HostCtx<()>,
    output: OutputInfo,
    services: &Services,
    wallpaper_control: &Control<WallpaperCommand>,
) {
    let logical = Size {
        width: output.logical.width,
        height: output.logical.height,
    };
    let buffer = buffer_size(logical, output.scale);
    let name = output
        .name
        .clone()
        .unwrap_or_else(|| output.id.0.to_string());
    let sent = wallpaper_control.send(WallpaperCommand::Prepare {
        output: OutputName(name),
        width: buffer.width.unsigned_abs(),
        height: buffer.height.unsigned_abs(),
    });
    if let Err(error) = sent {
        eprintln!("wallpaper_demo: preparing {:?}: {error}", output.name);
    }
    let config = wallpaper_layer(&output);
    // Every root needs `MEASURE`: without it, `Ds`'s own `use_rect`
    // reads (`OverlayHost`) panic the first time the renderer polls a woken task inside its
    // own render pass.
    let root = HostRoot::with_props(
        Wallpaper,
        WallpaperProps {
            output: output.clone(),
        },
    )
    .context(services.clone())
    .context(sill_surfaces::root::MEASURE);
    if let Err(error) = ctx.spawn_layer(config, root) {
        eprintln!("wallpaper: {error}");
    }
}
