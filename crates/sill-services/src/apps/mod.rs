//! The application index: desktop entries from every XDG applications directory, their icons
//! resolved once, rescanned on change. Plan hooks table, `use_apps`: freedesktop-desktop-entry
//! and freedesktop-icons; the theme from CosmicTk `icon_theme`, else Cosmic, else hicolor;
//! `notify` on the applications directories, 500 ms.

#![allow(unused_variables, dead_code)] // Freeze stubs

mod exec;
mod icons;
mod index;
mod match_app;
mod scan;

pub use exec::{ExecArgs, ExecError, expand_exec};
pub use icons::{IconTheme, icon_theme, resolve_icon};
pub use index::{
    App, AppIcon, AppIndex, AppsEvent, DbusActivation, DesktopActionEntry, Listing, Terminal,
    reduce_apps,
};
pub use match_app::match_app;
pub use scan::{applications_dirs, parse_entry, scan};

use std::time::Duration;

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// How long the applications directories must be quiet before a rescan.
pub const RESCAN_DEBOUNCE: Duration = Duration::from_millis(500);

/// What the apps service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppsCommand {
    /// Rescan now (`sill reload`).
    Rescan,
}

/// Scan once, publish, then watch every applications directory and rescan after
/// [`RESCAN_DEBOUNCE`] of quiet.
pub fn spawn(rt: &Handle) -> (watch::Receiver<AppIndex>, Control<AppsCommand>) {
    todo!(
        "M2 apps-index: scan() on the blocking pool, notify watch, AppsEvent::Scanned -> reduce_apps"
    )
}
