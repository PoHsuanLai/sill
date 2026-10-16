//! The program behind each window's app, as the compositor established it
//! (`quire_desktop_app_v1.subject`), projected for the windows' published state. Only the Quire
//! arm has any: the standard protocols say nothing of the program behind a window.

use desktop_client::DesktopState;
use sill_model::windows::Subjects;

/// The subject of each app id on screen, from the Quire windows that are joined to their
/// toplevels. An app id whose windows report different subjects, or only some of them one, has
/// none (`Subjects::collect`).
pub fn project(state: &DesktopState) -> Subjects {
    Subjects::collect(state.quire.windows().filter_map(|window| {
        let toplevel = state.toplevels.get(window.key)?;
        let subject = window
            .app
            .and_then(|key| state.quire.app(key))
            .and_then(|app| app.subject.as_ref())
            .map(|subject| subject.as_str().to_owned());
        Some((toplevel.app_id.clone(), subject))
    }))
}
