//! `Show`: open, close or toggle a panel by writing the link its surfaces follow. The panels are
//! `crate::actions::panel`'s: each opens on the output it picks.

use sill_ipc::response::Response;
use sill_ipc::show::{LauncherRequest, Reveal, ShowRequest};
use sill_model::actions::Panel;

use super::reach::Reach;
use crate::actions::panel;

/// The panel `request` names, shown or hidden.
pub fn dispatch(reach: &Reach, request: ShowRequest) -> Response {
    match request {
        ShowRequest::Launcher(request) => launcher(reach, request),
        ShowRequest::PowerMenu(reveal) => panel::show(reach, Panel::PowerMenu, reveal, None),
        ShowRequest::ForceQuit(reveal) => panel::show(reach, Panel::ForceQuit, reveal, None),
        ShowRequest::NotificationCenter(reveal) => {
            panel::show(reach, Panel::NotificationCenter, reveal, None)
        }
        ShowRequest::Companion(request) => crate::companion::dispatch(reach, request),
        ShowRequest::Voice(request) => crate::voice::dispatch(reach, request),
    }
}

/// The shared launcher state, which both launcher surfaces follow.
fn launcher(reach: &Reach, request: LauncherRequest) -> Response {
    match request {
        LauncherRequest::Close => panel::launcher(reach, Reveal::Close, String::new(), None),
        LauncherRequest::Open { query } => panel::launcher(reach, Reveal::Open, query, None),
    }
}
