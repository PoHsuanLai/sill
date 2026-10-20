//! Showing and hiding the shell's panels: the one place that picks the output a panel opens on
//! (the output the action came from, else [`Outputs::open_on`](crate::outputs::Outputs::open_on)'s
//! rule for the panel's surface) and says so when no output has the surface.

use shell_host::OutputId;
use sill_bar::Bar;
use sill_ipc::response::Response;
use sill_ipc::show::Reveal;
use sill_launcher::{Scope, SessionAction};
use sill_launcher_ui::Launcher;
use sill_model::actions::Panel;
use sill_notify_ui::NotificationCenter;
use sill_overlays::{EmptyTrashSheet, ForceQuit, PowerMenu};
use sill_ui_kit::links::Preselect;
use sill_ui_kit::surface::Surface;

use crate::ipc::dispatch::{Reach, unsupported};

/// `panel` shown, hidden or flipped by `reveal`, on `at` when the action came from an output,
/// else on the output the panel's own rule picks.
pub fn show(reach: &Reach, panel: Panel, reveal: Reveal, at: Option<OutputId>) -> Response {
    match panel {
        Panel::Launcher => launcher(reach, reveal, String::new(), at),
        Panel::PowerMenu => power_menu(reach, reveal, Preselect::Default, at),
        Panel::ForceQuit => force_quit(reach, reveal, at),
        Panel::NotificationCenter => notification_center(reach, reveal, at),
        Panel::ControlCenter => control_center(reach, at),
    }
}

/// The launcher, opened with `query` typed when `reveal` opens it.
pub fn launcher(reach: &Reach, reveal: Reveal, query: String, at: Option<OutputId>) -> Response {
    let link = &reach.links.launcher;
    match (reveal, output::<Launcher>(reach, at)) {
        (Reveal::Close, _) => link.close(),
        (Reveal::Toggle, Some(output)) => link.toggle(output),
        (Reveal::Open, Some(output)) => link.open(output, query),
        (Reveal::Toggle | Reveal::Open, None) => {
            return unsupported("no output has a launcher surface");
        }
    }
    Response::Done
}

/// The launcher, opened with nothing typed on `scope` alone.
pub fn launcher_in(reach: &Reach, scope: Scope, at: Option<OutputId>) -> Response {
    match output::<Launcher>(reach, at) {
        Some(output) => {
            reach.links.launcher.open_scoped(output, scope);
            Response::Done
        }
        None => unsupported("no output has a launcher surface"),
    }
}

/// The power menu, asking about `action` when it is a session action that asks first.
pub fn ask_session(reach: &Reach, action: SessionAction, at: Option<OutputId>) -> Response {
    power_menu(reach, Reveal::Open, Preselect::Action(action), at)
}

fn power_menu(
    reach: &Reach,
    reveal: Reveal,
    preselect: Preselect,
    at: Option<OutputId>,
) -> Response {
    let menu = &reach.links.power_menu;
    match (reveal, output::<PowerMenu>(reach, at)) {
        (Reveal::Close, _) => menu.close(),
        (Reveal::Open, Some(output)) => menu.open(output, preselect),
        (Reveal::Toggle, Some(output)) => menu.toggle(output),
        (Reveal::Open | Reveal::Toggle, None) => {
            return unsupported("no output has a power menu surface");
        }
    }
    Response::Done
}

fn force_quit(reach: &Reach, reveal: Reveal, at: Option<OutputId>) -> Response {
    let panel = &reach.links.force_quit;
    match (reveal, output::<ForceQuit>(reach, at)) {
        (Reveal::Close, _) => panel.close(),
        (Reveal::Open, Some(output)) => panel.open(output),
        (Reveal::Toggle, Some(output)) => panel.toggle(output),
        (Reveal::Open | Reveal::Toggle, None) => {
            return unsupported("no output has a Force Quit Applications surface");
        }
    }
    Response::Done
}

/// The Empty Trash question, asked on `at`, else on the focused window's output.
pub fn empty_trash(reach: &Reach, at: Option<OutputId>) -> Response {
    match output::<EmptyTrashSheet>(reach, at) {
        Some(output) => {
            reach.links.empty_trash.open(output);
            Response::Done
        }
        None => unsupported("no output has an Empty Trash surface"),
    }
}

fn notification_center(reach: &Reach, reveal: Reveal, at: Option<OutputId>) -> Response {
    let center = &reach.links.notification_center;
    match (reveal, output::<NotificationCenter>(reach, at)) {
        (Reveal::Close, _) => center.close(),
        (Reveal::Toggle, Some(output)) => center.toggle(output),
        (Reveal::Open, Some(output)) => center.open(output),
        (Reveal::Toggle | Reveal::Open, None) => {
            return unsupported("no output has a notification center surface");
        }
    }
    Response::Done
}

/// The control center is a popup of a bar: it toggles under that bar's item.
fn control_center(reach: &Reach, at: Option<OutputId>) -> Response {
    match output::<Bar>(reach, at) {
        Some(output) => {
            reach.links.control_center.toggle(output);
            Response::Done
        }
        None => unsupported("no output has a bar"),
    }
}

fn output<S: Surface>(reach: &Reach, at: Option<OutputId>) -> Option<OutputId> {
    at.or_else(|| reach.outputs.open_on::<S>(reach.services))
}
