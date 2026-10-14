//! `Show`: open, close or toggle a panel by writing the link its surfaces follow. Each panel
//! opens on the first output that has its surface, except Edit Widgets, which opens on the output
//! of the active window when it has the surface: the output under the pointer is not known to
//! the daemon.

use sill_bar::Bar;
use sill_ipc::response::Response;
use sill_ipc::show::{EditWidgetsRequest, LauncherRequest, Reveal, ShowRequest};
use sill_launcher_ui::Launcher;
use sill_notify_ui::NotificationCenter;
use sill_overlays::PowerMenu;
use sill_ui_kit::links::Preselect;
use sill_widgets_ui::EditWidgets;

use super::reach::Reach;
use super::reply::unsupported;
use crate::outputs::{OutputSurfaces, Outputs};
use shell_host::OutputId;
use sill_model::ids::OutputName;

/// The panel `request` names, shown or hidden.
pub fn dispatch(reach: &Reach, request: ShowRequest) -> Response {
    match request {
        ShowRequest::Launcher(request) => launcher(reach, request),
        ShowRequest::PowerMenu(reveal) => power_menu(reach, reveal),
        ShowRequest::NotificationCenter(reveal) => notification_center(reach, reveal),
        ShowRequest::ControlCenter => control_center(reach),
        ShowRequest::EditWidgets(request) => edit_widgets(reach, request),
        ShowRequest::Companion(request) => crate::companion::dispatch(reach, request),
        ShowRequest::Voice(request) => crate::voice::dispatch(reach, request),
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
    let link = &reach.links.launcher;
    match (
        request,
        first_with(reach.outputs, |s| s.surface_of::<Launcher>().is_some()),
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
        first_with(reach.outputs, |s| s.surface_of::<PowerMenu>().is_some()),
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
    let center = &reach.links.notification_center;
    let target = first_with(reach.outputs, |s| {
        s.surface_of::<NotificationCenter>().is_some()
    });
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
    match first_with(reach.outputs, |s| s.surface_of::<Bar>().is_some()) {
        Some(output) => {
            reach.links.control_center.toggle(output);
            Response::Done
        }
        None => unsupported("no output has a bar"),
    }
}

fn edit_widgets(reach: &Reach, request: EditWidgetsRequest) -> Response {
    let link = &reach.links.edit_widgets;
    let focus = reach
        .services
        .windows
        .state
        .borrow()
        .toplevels
        .focused_output();
    let with_surface: Vec<(OutputId, Option<String>)> = reach
        .outputs
        .outputs
        .iter()
        .filter(|(_, surfaces)| surfaces.surface_of::<EditWidgets>().is_some())
        .map(|(info, _)| (info.id, info.name.clone()))
        .collect();
    let target = focused_or_first(&with_surface, focus.as_ref());
    match (request, target) {
        (EditWidgetsRequest::Open, Some(output)) => link.open(output),
        (EditWidgetsRequest::Close, _) => link.close(),
        (EditWidgetsRequest::Open, None) => {
            return unsupported("no output has an Edit Widgets surface");
        }
    }
    Response::Done
}

/// The output named `focus` among `candidates` (an output's id and name), else the first.
pub(crate) fn focused_or_first(
    candidates: &[(OutputId, Option<String>)],
    focus: Option<&OutputName>,
) -> Option<OutputId> {
    candidates
        .iter()
        .find(|(_, name)| focus.is_some_and(|focus| name.as_deref() == Some(focus.0.as_str())))
        .or_else(|| candidates.first())
        .map(|(id, _)| *id)
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;
    use sill_model::ids::OutputName;

    use super::focused_or_first;

    #[test]
    fn edit_widgets_opens_where_the_active_window_is_else_on_the_first_output() {
        let outputs = [
            (OutputId(1), Some("DP-1".to_owned())),
            (OutputId(2), Some("HDMI-A-1".to_owned())),
            (OutputId(3), None),
        ];
        let on = |name: &str| Some(OutputName(name.to_owned()));
        let cases = [
            (
                "the active window's output",
                on("HDMI-A-1"),
                Some(OutputId(2)),
            ),
            (
                "the first when it is on the first",
                on("DP-1"),
                Some(OutputId(1)),
            ),
            ("the first with no active window", None, Some(OutputId(1))),
            (
                "the first for an output with no surface",
                on("eDP-1"),
                Some(OutputId(1)),
            ),
        ];
        for (name, focus, want) in cases {
            assert_eq!(focused_or_first(&outputs, focus.as_ref()), want, "{name}");
        }
        assert_eq!(
            focused_or_first(&[], on("DP-1").as_ref()),
            None,
            "no output has it"
        );
    }
}
