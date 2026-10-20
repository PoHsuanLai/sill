//! What the dock's menus read from the person's folders and change there: whether the Trash
//! holds anything, whether an app starts at login, and turning that on or off. The folders are
//! the process's own (`Dirs::from_process`); the changes run off the UI thread.

use ds::prelude::Availability;
use sill_launcher::AppId;
use sill_services::autostart;
use sill_services::env::Dirs;
use sill_services::trash;

use crate::dock::context_menu::Checked;
use crate::dock::pick::LoginChange;

/// Whether Empty Trash can be chosen: not while the Trash is empty.
pub(crate) fn empty_trash_availability() -> Availability {
    if trash::is_empty(&Dirs::from_process()) {
        Availability::Disabled
    } else {
        Availability::Enabled
    }
}

/// Whether `app` starts at login, as the autostart folder says; an unknown app does not.
pub(crate) fn login_state(app: Option<&AppId>) -> Checked {
    match app {
        Some(app) if autostart::login_of(&Dirs::from_process(), app).is_on() => Checked::Checked,
        Some(_) | None => Checked::Unchecked,
    }
}

/// Carry out `change` on a thread of its own.
pub(crate) fn change_login(change: LoginChange) {
    let dirs = Dirs::from_process();
    match change {
        LoginChange::On { app, entry } => autostart::turn_on_in_background(dirs, app, entry),
        LoginChange::Off { app } => autostart::turn_off_in_background(dirs, app),
    }
}
