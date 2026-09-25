//! The daemon's M5 half: one power menu surface per output, spawned warm and hidden (the
//! launcher's pattern), and the `power-menu` and `control-center` requests, which only write the
//! shared links the surfaces follow ([`crate::links`]).

use shell_host::{HostCtx, OutputInfo, Root};
use sill_ipc::{ControlCenterRequest, PowerMenuRequest, Response};
use sill_surfaces::surfaces::power_menu::{PowerMenuProps, Preselect, power_menu_layer};

use crate::links::Links;
use crate::msg::ShellMsg;
use crate::outputs::Outputs;

/// Spawn a hidden power menu for every output whose row has none yet, each root built by `root`
/// (the daemon's `surface_root`, which adds the links).
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
    for info in missing {
        let config = power_menu_layer(&info);
        let namespace = config.namespace.clone();
        let props = PowerMenuProps {
            output: info.clone(),
        };
        match ctx.spawn_layer(
            config,
            root(Root::with_props(sill_surfaces::PowerMenu, props)),
        ) {
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

/// `sill power-menu open|close|toggle`: on the first output with a power menu (the output under
/// the pointer is not known to the daemon; FINDINGS F140).
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

/// `sill control-center toggle`: asks the first output's bar (it does nothing until the bar
/// follows the link, M5 agent c).
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
