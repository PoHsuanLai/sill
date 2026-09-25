//! The M5 shared channels every surface root gets beside `Services` and `SettingsFeed`: the focus
//! mode, the power menu's shown state, and the control center's toggle asks (FINDINGS "M5
//! freeze").
//!
//! Set once in [`crate::daemon::run`] and read by `daemon::surface_root` and the IPC dispatch.
//! A `OnceLock` rather than one more argument through every `spawn_missing_*`: those functions
//! are shared with work in flight on other branches, and a root without these contexts (a
//! headless render, a test) is still valid, since every consumer reads them with
//! `try_use_context` or is not mounted there.

use std::sync::OnceLock;

use sill_settings::FocusLink;
use sill_surfaces::surfaces::control_center::ControlCenterLink;
use sill_surfaces::surfaces::power_menu::PowerMenuLink;

/// The daemon's M5 links.
#[derive(Debug, Clone)]
pub struct Links {
    pub focus: FocusLink,
    pub power_menu: PowerMenuLink,
    pub control_center: ControlCenterLink,
}

static LINKS: OnceLock<Links> = OnceLock::new();

/// Keep `links` for the process; a second call is ignored (there is one daemon per process).
pub fn install(links: Links) {
    let _first_wins = LINKS.set(links);
}

/// The links, once installed.
pub fn get() -> Option<&'static Links> {
    LINKS.get()
}
