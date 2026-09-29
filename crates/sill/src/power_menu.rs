//! The daemon's power menu half: one power menu surface per output, spawned warm and hidden (the
//! launcher's pattern), and the `power-menu` and `control-center` requests, which only write the
//! shared links the surfaces follow ([`crate::links`]).
//!
//! **Dry run.** A chosen action goes to the session service, which takes it through the daemon's
//! `SessionActions` (`sill_services::env`): logind's, or, in a debug build started with `sill run
//! --session-dry-run RECORD`, a dry run that only prints and records it. Each surface prints
//! `power_menu.backend=dry-run` as it mounts in that case, which `dev/accept-power-menu.sh` checks
//! before it presses Enter.
//!
//! **Scripted keys.** On the nested cosmic-comp the power menu gets no keyboard focus, and
//! shell-host injects no keys, so a debug build started with `--power-menu-keys FIFO` (only
//! beside `--session-dry-run`, so a scripted Enter can never reach logind) also feeds each line
//! written to it (`right`, `enter`, ...) to every power menu as a key ([`key_script`]).

use shell_host::{HostCtx, OutputInfo, Root};
use sill_ipc::{ControlCenterRequest, PowerMenuRequest, Response};
use sill_surfaces::surfaces::power_menu::{PowerMenuProps, Preselect, power_menu_layer};

use crate::links::Links;
use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden power menu for every output whose row has none yet, each root built by `root`
/// (the daemon's `surface_root`, which adds the links), with the scripted keys when there are.
pub fn spawn_missing(
    ctx: &mut HostCtx<ShellMsg>,
    outputs: &mut Outputs,
    root: impl Fn(Root) -> Root,
) {
    let missing: Vec<OutputInfo> = outputs
        .outputs
        .iter()
        .filter(|(_, surfaces)| surfaces.power_menu.is_none())
        .map(|(info, _)| info.clone())
        .collect();
    let keys = crate::links::get().and_then(|links| links.power_menu_keys.clone());
    for info in missing {
        let config = power_menu_layer(&info);
        let namespace = config.namespace.clone();
        let props = PowerMenuProps {
            output: info.clone(),
        };
        let surface = root(Root::with_props(sill_surfaces::PowerMenu, props));
        let surface = match &keys {
            Some(keys) => surface.context(keys.clone()),
            None => surface,
        };
        match ctx.spawn_layer(config, surface) {
            Ok(id) => {
                ctx.set_visibility(id, shell_host::Visibility::Hidden);
                let mut surfaces = outputs.get(info.id).cloned().unwrap_or_default();
                surfaces.power_menu = Some(id);
                outputs.set(info, surfaces);
            }
            Err(error) => eprintln!("sill: spawning {namespace} on {:?}: {error}", info.name),
        }
    }
}

/// The keys every power menu follows, read from the FIFO at `path` on a thread of their own.
#[cfg(feature = "debug")]
pub fn key_script(
    path: std::path::PathBuf,
) -> sill_surfaces::surfaces::power_menu::PowerMenuScript {
    use sill_surfaces::surfaces::power_menu::{PowerMenuScript, ScriptedKey};
    let (tx, rx) = tokio::sync::watch::channel(ScriptedKey::default());
    eprintln!("sill: power menu: scripted keys from {}", path.display());
    let spawned = std::thread::Builder::new()
        .name("power-menu-keys".to_owned())
        .spawn(move || read_keys(&path, &tx));
    if let Err(error) = spawned {
        eprintln!("sill: power menu: no key script: {error}");
    }
    PowerMenuScript(rx)
}

/// Read `path` a line at a time, forever (a FIFO ends at each writer's close and is opened
/// again), sending each key it names.
#[cfg(feature = "debug")]
fn read_keys(
    path: &std::path::Path,
    tx: &tokio::sync::watch::Sender<sill_surfaces::surfaces::power_menu::ScriptedKey>,
) {
    use sill_surfaces::surfaces::power_menu::script_key;
    use std::io::BufRead;
    loop {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) => {
                eprintln!("sill: power menu: key script {}: {error}", path.display());
                return;
            }
        };
        for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
            match script_key(&line) {
                Some(key) => tx.send_modify(|scripted| {
                    scripted.count += 1;
                    scripted.key = Some(key);
                }),
                None => eprintln!("sill: power menu: no key {line:?}"),
            }
        }
    }
}

/// `sill power-menu open|close|toggle`: on the first output with a power menu (the output under
/// the pointer is not known to the daemon).
pub fn dispatch_power_menu(
    outputs: &Outputs,
    links: Option<&Links>,
    request: PowerMenuRequest,
) -> Response {
    let Some(links) = links else {
        return unsupported("the power menu's link is not installed");
    };
    let target = outputs
        .outputs
        .iter()
        .find(|(_, s)| s.power_menu.is_some())
        .map(|(info, _)| info.id);
    match (request, target) {
        (PowerMenuRequest::Close, _) => links.power_menu.close(),
        (PowerMenuRequest::Open, Some(output)) => links.power_menu.open(output, Preselect::Default),
        (PowerMenuRequest::Toggle, Some(output)) => links.power_menu.toggle(output),
        (PowerMenuRequest::Open | PowerMenuRequest::Toggle, None) => {
            return unsupported("no output has a power menu surface");
        }
    }
    Response::Done
}

/// `sill control-center toggle`: asks the first output's bar.
pub fn dispatch_control_center(
    outputs: &Outputs,
    links: Option<&Links>,
    request: ControlCenterRequest,
) -> Response {
    let Some(links) = links else {
        return unsupported("the control center's link is not installed");
    };
    let target = outputs
        .outputs
        .iter()
        .find(|(_, s)| s.bar.is_some())
        .map(|(info, _)| info.id);
    match (request, target) {
        (ControlCenterRequest::Toggle, Some(output)) => links.control_center.toggle(output),
        (ControlCenterRequest::Toggle, None) => return unsupported("no output has a bar"),
    }
    Response::Done
}

fn unsupported(message: &str) -> Response {
    Response::Failed(sill_ipc::Failure {
        kind: sill_ipc::FailureKind::Unsupported,
        message: message.to_owned(),
    })
}
