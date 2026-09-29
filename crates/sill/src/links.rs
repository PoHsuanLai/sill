//! The shared channels every surface root gets beside `Services` and `SettingsFeed`: the focus
//! mode, the power menu's shown state, and the control center's toggle asks; the notification
//! center's shown state and the banners' output; the hot corners' firings; the switcher's asks and
//! the lock prompt's output.
//!
//! Set once in [`crate::daemon::run`] and read by `daemon::surface_root` and the IPC dispatch.
//! A `OnceLock` rather than one more argument through every `spawn_missing_*`: those functions
//! are shared with work in flight on other branches, and a root without these contexts (a
//! headless render, a test) is still valid, since every consumer reads them with
//! `try_use_context` or is not mounted there.

use std::sync::OnceLock;

use sill_settings::FocusLink;
use sill_surfaces::surfaces::banners::BannerLink;
use sill_surfaces::surfaces::control_center::ControlCenterLink;
use sill_surfaces::surfaces::edit_widgets::{EditWidgetsLink, WidgetsWriter};
use sill_surfaces::surfaces::hot_corners::CornerLink;
use sill_surfaces::surfaces::lock::LockLink;
use sill_surfaces::surfaces::notification_center::CenterLink;
use sill_surfaces::surfaces::power_menu::PowerMenuLink;
use sill_surfaces::surfaces::switcher::SwitcherLink;

/// The daemon's links.
#[derive(Debug, Clone)]
pub struct Links {
    pub focus: FocusLink,
    pub power_menu: PowerMenuLink,
    pub control_center: ControlCenterLink,
    pub notification_center: CenterLink,
    pub banners: BannerLink,
    /// Every corner surface writes its firings here; the daemon carries them out. The
    /// screenshot thumbnail follows `banners` (it shows on the banners' output).
    pub corners: CornerLink,
    /// `sill switcher ...` asks every switcher surface here; the lock surfaces follow
    /// `lock` for which one shows the password prompt.
    pub switcher: SwitcherLink,
    pub lock: LockLink,
    /// Edit Widgets: which output shows it (the notification center's "Edit Widgets", `sill
    /// widgets edit`), and where its changes are written (`crate::widgets::writer`).
    pub edit_widgets: EditWidgetsLink,
    pub widgets_writer: WidgetsWriter,
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
