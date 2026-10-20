//! The preferences an action flips: the dock's auto-hide and magnification and Do Not Disturb. Both are written to
//! `settings.toml`, which stays the one truth; the surfaces follow once the file settles.

use ds_settings::Store;
use sill_ipc::response::{FailureKind, Response};
use sill_model::actions::Pref;
use sill_settings::{AutoHide, FocusMode, Magnification, ShellFile};
use sill_ui_kit::links::Delivery;
use tokio::sync::watch;

use crate::daemon::settings::write_key;
use crate::ipc::dispatch::{Reach, failed};

/// Flip `pref`.
pub fn flip(reach: &Reach, pref: Pref) -> Response {
    match pref {
        Pref::DockAutoHide => toggle_dock_hide(reach.store, reach.settings),
        Pref::DockMagnification => toggle_dock_magnification(reach.store, reach.settings),
        Pref::Focus => toggle_focus(reach),
    }
}

/// Flip `dock.autohide` in `settings.toml`, and publish the file at once so the dock follows
/// before the watch has fired.
fn toggle_dock_hide(store: &Store, settings: &watch::Sender<ShellFile>) -> Response {
    let now = match crate::daemon::settings::load(store).dock.autohide {
        AutoHide::Off => AutoHide::On,
        AutoHide::On => AutoHide::Off,
    };
    if let Err(error) = write_key(store, "dock.autohide", &now) {
        return failed(
            FailureKind::Internal,
            format!("saving the settings: {error}"),
        );
    }
    settings.send_replace(crate::daemon::settings::load(store));
    Response::Done
}

/// Flip `dock.magnification` the same way.
fn toggle_dock_magnification(store: &Store, settings: &watch::Sender<ShellFile>) -> Response {
    let now = match crate::daemon::settings::load(store).dock.magnification {
        Magnification::On => Magnification::Off,
        Magnification::Off => Magnification::On,
    };
    if let Err(error) = write_key(store, "dock.magnification", &now) {
        return failed(
            FailureKind::Internal,
            format!("saving the settings: {error}"),
        );
    }
    settings.send_replace(crate::daemon::settings::load(store));
    Response::Done
}

/// Turn Do Not Disturb on when it is off, off when it is on.
fn toggle_focus(reach: &Reach) -> Response {
    let focus = &reach.links.focus;
    let next = match *focus.mode().borrow() {
        FocusMode::Off => FocusMode::DoNotDisturb,
        FocusMode::DoNotDisturb => FocusMode::Off,
    };
    match focus.set(next) {
        Delivery::Queued => Response::Done,
        Delivery::Gone => failed(
            FailureKind::Internal,
            "the focus setting has stopped".to_owned(),
        ),
    }
}
