//! `Show`: open, close or toggle a panel by writing the link its surfaces follow. Each panel
//! opens on the first output that has its surface: the output under the pointer is not known to
//! the daemon.

use sill_ipc::response::Response;
use sill_ipc::show::{EditWidgetsRequest, LauncherRequest, Reveal, ShowRequest};
use sill_surfaces::surfaces::power_menu::Preselect;

use super::reach::Reach;
use super::reply::unsupported;
use crate::outputs::{OutputSurfaces, Outputs};
use shell_host::OutputId;

/// The panel `request` names, shown or hidden.
pub fn dispatch(reach: &Reach, request: ShowRequest) -> Response {
    match request {
        ShowRequest::Launcher(request) => launcher(reach, request),
        ShowRequest::PowerMenu(reveal) => power_menu(reach, reveal),
        ShowRequest::NotificationCenter(reveal) => notification_center(reach, reveal),
        ShowRequest::ControlCenter => control_center(reach),
        ShowRequest::EditWidgets(request) => edit_widgets(reach, request),
    }
}

/// The first output whose row satisfies `has`.
fn first_with(outputs: &Outputs, has: impl Fn(&OutputSurfaces) -> bool) -> Option<OutputId> {
    outputs
        .outputs
        .iter()
        .find(|(_, surfaces)| has(surfaces))
        .map(|(info, _)| info.id)
}

/// The shared launcher state, which both launcher surfaces follow.
fn launcher(reach: &Reach, request: LauncherRequest) -> Response {
    let link = reach.launcher;
    match (request, first_with(reach.outputs, |s| s.launcher.is_some())) {
        (LauncherRequest::Close, _) => link.close(),
        (LauncherRequest::Toggle, Some(output)) => link.toggle(output),
        (LauncherRequest::Open { query }, Some(output)) => link.open(output, query),
        (LauncherRequest::Toggle | LauncherRequest::Open { .. }, None) => {
            return unsupported("no output has a launcher surface");
        }
    }
    Response::Done
}

fn power_menu(reach: &Reach, reveal: Reveal) -> Response {
    let Some(links) = reach.links else {
        return unsupported("the power menu's link is not installed");
    };
    let menu = &links.power_menu;
    match (
        reveal,
        first_with(reach.outputs, |s| s.power_menu.is_some()),
    ) {
        (Reveal::Close, _) => menu.close(),
        (Reveal::Open, Some(output)) => menu.open(output, Preselect::Default),
        (Reveal::Toggle, Some(output)) => menu.toggle(output),
        (Reveal::Open | Reveal::Toggle, None) => {
            return unsupported("no output has a power menu surface");
        }
    }
    Response::Done
}

fn notification_center(reach: &Reach, reveal: Reveal) -> Response {
    let center = reach.links.map(|links| &links.notification_center);
    let target = first_with(reach.outputs, |s| s.notification_center.is_some());
    match (reveal, center, target) {
        (Reveal::Close, Some(center), _) => center.close(),
        (Reveal::Toggle, Some(center), Some(output)) => center.toggle(output),
        (Reveal::Open, Some(center), Some(output)) => center.open(output),
        (Reveal::Close, None, _) | (Reveal::Toggle | Reveal::Open, _, _) => {
            return unsupported("no output has a notification center surface");
        }
    }
    Response::Done
}

/// The control center is a popup of a bar: it toggles under the first bar's item.
fn control_center(reach: &Reach) -> Response {
    let Some(links) = reach.links else {
        return unsupported("the control center's link is not installed");
    };
    match first_with(reach.outputs, |s| s.bar.is_some()) {
        Some(output) => {
            links.control_center.toggle(output);
            Response::Done
        }
        None => unsupported("no output has a bar"),
    }
}

fn edit_widgets(reach: &Reach, request: EditWidgetsRequest) -> Response {
    let link = reach.links.map(|links| &links.edit_widgets);
    let target = first_with(reach.outputs, |s| s.edit_widgets.is_some());
    match (request, link, target) {
        (EditWidgetsRequest::Open, Some(link), Some(output)) => link.open(output),
        (EditWidgetsRequest::Close, Some(link), _) => link.close(),
        (EditWidgetsRequest::Open | EditWidgetsRequest::Close, _, _) => {
            return unsupported("no output has an Edit Widgets surface");
        }
    }
    Response::Done
}
