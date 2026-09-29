//! The launcher on a real compositor with fake services, for manual runs and the acceptance
//! (`dev/accept/launcher.sh`): a catcher and a panel per output, spawned hidden as the
//! daemon spawns them, over a fake search runner (`tests/launcher_support`), driven by lines
//! on stdin:
//!
//! - `toggle`, `open [QUERY]`, `close`: what `sill launcher ...` does;
//! - anything else is a shell-host `debug-inject` command (`sill-launcher-<output> key
//!   ArrowDown`, `sill-launcher-<output> key k+ctrl`, `sill-launcher-<output> capture PATH`,
//!   `sill-launcher-catcher-<output> click X Y`, `stats`, `quit`); a key enters the panel where a
//!   real one does.
//!
//! What the launcher sends to the fake launch and compositor services is printed on stderr.
//!
//! `cargo run -p sill-ui-kit --example launcher_demo`

#[path = "../tests/launcher_support/mod.rs"]
mod launcher_support;

use std::fmt::Debug;
use std::io::BufRead;

use shell_host::{
    Host, HostCtx, HostEvent, HostRoot, LayerConfig, OutputId, OutputInfo, SurfaceId, Visibility,
};
use sill_settings::{SettingsFeed, ShellFile};
use sill_ui_kit::links::LauncherLink;
use sill_ui_kit::surface::{MountProps, Surface, mount};
use sill_ui_kit::{Launcher, LauncherCatcher};
use tokio::sync::mpsc;

/// A line from stdin, or its end.
enum DemoMsg {
    Line(String),
    End,
}

/// What the demo's roots share.
struct Demo {
    services: sill_ui_kit::services::Services,
    settings: SettingsFeed,
    link: LauncherLink,
    providers: sill_ui_kit::surfaces::launcher::LauncherProviders,
    panels: Vec<(OutputId, SurfaceId)>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("launcher_demo: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (services, feeds) = launcher_support::services();
    print_commands("launch", feeds.bar.launch);
    print_commands("wl", feeds.bar.wl);
    let mut demo = Demo {
        services,
        settings: SettingsFeed::fixed(ShellFile::default()),
        link: LauncherLink::new(),
        providers: launcher_support::providers(),
        panels: Vec::new(),
    };
    let host =
        Host::<DemoMsg>::connect(sill_ui_kit::root::host_config()).map_err(|e| e.to_string())?;
    read_stdin(host.handle());
    let _keep_results = feeds.results;
    host.run(move |ctx, event| match event {
        HostEvent::Started => {
            for output in ctx.outputs() {
                spawn_launcher(ctx, output, &mut demo);
            }
        }
        HostEvent::OutputAdded(output) => spawn_launcher(ctx, output, &mut demo),
        HostEvent::KeyboardFocus { id, focus } if demo.panels.iter().any(|(_, p)| *p == id) => {
            demo.link.focus(focus);
        }
        HostEvent::App(DemoMsg::Line(line)) => run_line(ctx, &demo, &line),
        HostEvent::App(DemoMsg::End) => ctx.exit(),
        _ => {}
    })
    .map_err(|e| e.to_string())
}

/// The catcher, then the panel, on `output`, both hidden at once (the daemon's order).
fn spawn_launcher(ctx: &mut HostCtx<DemoMsg>, output: OutputInfo, demo: &mut Demo) {
    if demo.panels.iter().any(|(id, _)| *id == output.id) {
        return;
    }
    let file = ShellFile::default();
    let root = |root: HostRoot| {
        root.context(demo.services.clone())
            .context(demo.settings.clone())
            .context(sill_ui_kit::root::MEASURE)
            .context(sill_ui_kit::root::FOCUS)
            .context(demo.link.clone())
            .context(demo.providers.clone())
    };
    let catcher = root(HostRoot::with_props(
        mount::<LauncherCatcher>,
        MountProps::<LauncherCatcher>::new(output.clone(), ()),
    ));
    spawn_hidden(ctx, LauncherCatcher::layer(&output, &file, ()), catcher);
    let config = Launcher::layer(&output, &file, ());
    let panel = root(HostRoot::with_props(
        mount::<Launcher>,
        MountProps::<Launcher>::new(output.clone(), ()),
    ));
    if let Some(id) = spawn_hidden(ctx, config, panel) {
        demo.panels.push((output.id, id));
    }
}

fn spawn_hidden(
    ctx: &mut HostCtx<DemoMsg>,
    config: LayerConfig,
    root: HostRoot,
) -> Option<SurfaceId> {
    match ctx.spawn_layer(config, root) {
        Ok(id) => {
            ctx.set_visibility(id, Visibility::Hidden);
            Some(id)
        }
        Err(error) => {
            eprintln!("launcher_demo: {error}");
            None
        }
    }
}

/// One stdin line.
fn run_line(ctx: &mut HostCtx<DemoMsg>, demo: &Demo, line: &str) {
    let output = demo.panels.first().map(|(id, _)| *id);
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    match (word, output) {
        ("toggle", Some(output)) => demo.link.toggle(output),
        ("open", Some(output)) => demo.link.open(output, rest.to_owned()),
        ("close", _) => demo.link.close(),
        _ => inject(ctx, line),
    }
}

/// One `debug-inject` line.
fn inject(ctx: &mut HostCtx<DemoMsg>, line: &str) {
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

/// Print every command the launcher sends on `rx`, on its own thread.
fn print_commands<C: Debug + Send + 'static>(
    name: &'static str,
    mut rx: mpsc::UnboundedReceiver<C>,
) {
    std::thread::spawn(move || {
        while let Some(command) = rx.blocking_recv() {
            eprintln!("launcher_demo: {name} {command:?}");
        }
    });
}
