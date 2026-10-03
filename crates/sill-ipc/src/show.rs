//! The panels a client opens and closes: the launcher, the power menu, the control center, the
//! notification center and Edit Widgets. Each opens on the output the daemon picks (the output
//! under the pointer is not known to the daemon), which is the first with that surface; Edit
//! Widgets prefers the output of the active window.

use serde::{Deserialize, Serialize};

/// `sill launcher|power-menu|control-center|notifications|widgets ...`, the verbs that show or
/// hide something.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ShowRequest {
    /// Show, hide or seed the launcher.
    Launcher(LauncherRequest),
    /// The power menu (`sill power-menu {open,close,toggle}`).
    PowerMenu(Reveal),
    /// The notification center (`sill notifications {toggle,open,close}`).
    NotificationCenter(Reveal),
    /// The control center under the first bar's item: it is a popup of a bar, so it has no open
    /// of its own to seed. Opens when closed, closes when open.
    ControlCenter,
    /// Edit Widgets, the widget gallery (`sill widgets {edit,edit-close}`).
    EditWidgets(EditWidgetsRequest),
}

/// How a panel changes: the shortcut that flips it, or the two that set it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reveal {
    /// Open when closed, close when open.
    Toggle,
    /// Open, or stay open.
    Open,
    /// Close if open; nothing otherwise.
    Close,
}

/// The launcher's three verbs (`sill launcher {toggle,open [--query],close}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LauncherRequest {
    /// Open when closed, close when open: the COSMIC `Launcher` shortcut.
    Toggle,
    /// Open with `query` already typed; an empty query opens a fresh launcher.
    Open { query: String },
    /// Close if open; nothing otherwise.
    Close,
}

/// Edit Widgets has no toggle: it opens on the output of the active window when that output has
/// its surface, else on the first that has, and closes wherever it is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditWidgetsRequest {
    Open,
    Close,
}
