//! `Show`: open, close or toggle a panel by writing the link its surfaces follow. Each panel
//! opens on the output [`panel_output`] picks.

use sill_bar::Bar;
use sill_ipc::response::Response;
use sill_ipc::show::{LauncherRequest, Reveal, ShowRequest};
use sill_launcher_ui::Launcher;
use sill_notify_ui::NotificationCenter;
use sill_overlays::{ForceQuit, PowerMenu};
use sill_ui_kit::links::Preselect;

use super::panel_output::panel_output;
use super::reach::Reach;
use super::reply::unsupported;

/// The panel `request` names, shown or hidden.
pub fn dispatch(reach: &Reach, request: ShowRequest) -> Response {
    match request {
        ShowRequest::Launcher(request) => launcher(reach, request),
        ShowRequest::PowerMenu(reveal) => power_menu(reach, reveal),
        ShowRequest::ForceQuit(reveal) => force_quit(reach, reveal),
        ShowRequest::NotificationCenter(reveal) => notification_center(reach, reveal),
        ShowRequest::ControlCenter => control_center(reach),
        ShowRequest::Companion(request) => crate::companion::dispatch(reach, request),
        ShowRequest::Voice(request) => crate::voice::dispatch(reach, request),
    }
}

/// The shared launcher state, which both launcher surfaces follow.
fn launcher(reach: &Reach, request: LauncherRequest) -> Response {
    let link = &reach.links.launcher;
    match (
        request,
        panel_output::<Launcher>(reach.outputs, reach.services),
    ) {
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
    let menu = &reach.links.power_menu;
    match (
        reveal,
        panel_output::<PowerMenu>(reach.outputs, reach.services),
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

fn force_quit(reach: &Reach, reveal: Reveal) -> Response {
    let panel = &reach.links.force_quit;
    match (
        reveal,
        panel_output::<ForceQuit>(reach.outputs, reach.services),
    ) {
        (Reveal::Close, _) => panel.close(),
        (Reveal::Open, Some(output)) => panel.open(output),
        (Reveal::Toggle, Some(output)) => panel.toggle(output),
        (Reveal::Open | Reveal::Toggle, None) => {
            return unsupported("no output has a Force Quit Applications surface");
        }
    }
    Response::Done
}

fn notification_center(reach: &Reach, reveal: Reveal) -> Response {
    let center = &reach.links.notification_center;
    let target = panel_output::<NotificationCenter>(reach.outputs, reach.services);
    match (reveal, target) {
        (Reveal::Close, _) => center.close(),
        (Reveal::Toggle, Some(output)) => center.toggle(output),
        (Reveal::Open, Some(output)) => center.open(output),
        (Reveal::Toggle | Reveal::Open, None) => {
            return unsupported("no output has a notification center surface");
        }
    }
    Response::Done
}

/// The control center is a popup of a bar: it toggles under the first bar's item.
fn control_center(reach: &Reach) -> Response {
    match panel_output::<Bar>(reach.outputs, reach.services) {
        Some(output) => {
            reach.links.control_center.toggle(output);
            Response::Done
        }
        None => unsupported("no output has a bar"),
    }
}
