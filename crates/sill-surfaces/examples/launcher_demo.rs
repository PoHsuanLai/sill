//! The launcher on a real compositor with fake services, for manual runs and the acceptance
//! (`dev/accept-launcher-ui.sh`): a catcher and a panel per output, spawned hidden as the
//! daemon spawns them, over a fake search runner (`tests/launcher_support`), driven by lines
//! on stdin:
//!
//! - `toggle`, `open [QUERY]`, `close`: what `sill launcher ...` does;
//! - `type TEXT`: the field's text becomes TEXT (shell-host injects no keys, FINDINGS G20);
//! - `key tab|shift-tab|actions|escape`: those keys, through the panel's keyboard machine;
//! - anything else is a shell-host `debug-inject` command (`sill-launcher-<output> capture
//!   PATH`, `sill-launcher-catcher-<output> click X Y`, `stats`, `quit`).
//!
//! What the launcher sends to the fake launch and compositor services is printed on stderr.
//!
//! `cargo run -p sill-surfaces --example launcher_demo`

#[path = "../tests/launcher_support/mod.rs"]
mod launcher_support;

use std::fmt::Debug;
use std::io::BufRead;

use shell_host::{
    Host, HostConfig, HostCtx, HostEvent, LayerConfig, OutputId, OutputInfo, Root, SurfaceId,
    Visibility,
};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::surfaces::launcher::script::{LauncherScript, Scripted, ScriptedInput};
use sill_surfaces::surfaces::launcher::{
    LauncherCatcherProps, LauncherKey, LauncherLink, LauncherProps, catcher_layer, panel_layer,
};
use sill_surfaces::{Launcher, LauncherCatcher};
use tokio::sync::{mpsc, watch};

/// A line from stdin, or its end.
enum DemoMsg {
    Line(String),
    End,
}

/// What the demo's roots share.
struct Demo {
    services: sill_services::Services,
    settings: SettingsFeed,
    link: LauncherLink,
    script: watch::Sender<Scripted>,
    providers: sill_surfaces::surfaces::launcher::LauncherProviders,
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
    let (script, _) = watch::channel(Scripted::default());
    let mut demo = Demo {
        services,
        settings: SettingsFeed::fixed(ShellFile::default()),
        link: LauncherLink::new(),
        script,
        providers: launcher_support::providers(),
        panels: Vec::new(),
    };
    let host = Host::<DemoMsg>::connect(HostConfig::default()).map_err(|e| e.to_string())?;
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
    let root = |root: Root| {
        root.context(demo.services.clone())
            .context(demo.settings.clone())
            .context(sill_surfaces::MEASURE)
            .context(sill_surfaces::FOCUS)
            .context(demo.link.clone())
            .context(demo.providers.clone())
            .context(LauncherScript(demo.script.subscribe()))
    };
    let catcher = root(Root::with_props(
        LauncherCatcher,
        LauncherCatcherProps {
            output: output.clone(),
        },
    ));
    spawn_hidden(ctx, catcher_layer(&output), catcher);
    let config = panel_layer(&output, &file.bar, &file.launcher);
    let panel = root(Root::with_props(
        Launcher,
        LauncherProps {
            output: output.clone(),
        },
    ));
    if let Some(id) = spawn_hidden(ctx, config, panel) {
        demo.panels.push((output.id, id));
    }
}

fn spawn_hidden(ctx: &mut HostCtx<DemoMsg>, config: LayerConfig, root: Root) -> Option<SurfaceId> {
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
        ("type", _) => script(demo, ScriptedInput::Type(rest.to_owned())),
        ("key", _) => match key(rest) {
            Some(key) => script(demo, ScriptedInput::Key(key)),
            None => eprintln!("launcher_demo: no key {rest:?}"),
        },
        _ => inject(ctx, line),
    }
}

fn key(word: &str) -> Option<LauncherKey> {
    match word.trim() {
        "tab" => Some(LauncherKey::Tab),
        "shift-tab" => Some(LauncherKey::ShiftTab),
        "actions" => Some(LauncherKey::Actions),
        "escape" => Some(LauncherKey::Escape),
        _ => None,
    }
}

fn script(demo: &Demo, input: ScriptedInput) {
    demo.script.send_modify(|scripted| {
        scripted.count += 1;
        scripted.input = Some(input);
    });
}

/// One `debug-inject` line.
fn inject(ctx: &mut HostCtx<DemoMsg>, line: &str) {
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
