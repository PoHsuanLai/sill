//! What activating a result asks the daemon to do. A closed vocabulary: the daemon matches it
//! exhaustively, so a new kind of activation is a compile error at every site that must act.

use serde::{Deserialize, Serialize};

use crate::ids::AppId;

/// The effect of choosing a result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Activation {
    /// Start (or D-Bus activate) an app, optionally one of its desktop actions.
    Launch { app: AppId, action: DesktopAction },
    /// Activate an open window.
    FocusWindow { window: WindowKey },
    /// Put text on the clipboard (the calculator's result).
    Copy { text: String },
    /// Open a settings page.
    OpenSettings { target: SettingsTarget },
    /// A session command.
    System(SystemCommand),
    /// Nothing to do (a header row, a stale result).
    Nothing,
}

/// Which entry point of an app to start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DesktopAction {
    /// The entry's own `Exec`.
    Default,
    /// `[Desktop Action <id>]`.
    Named(String),
}

/// A window, by the key the `cosmic_wl` thread gave its toplevel handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowKey(pub u64);

/// A settings destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum SettingsTarget {
    /// A `cosmic-settings` page id (`cosmic-settings <page>`), when that binary exists.
    Cosmic(String),
    /// A quire settings key path (`appearance.theme`), resolved through the schemas
    /// (design/22-SETTINGS.md §9.3 "deep links").
    Key(String),
}

/// The system provider's commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemCommand {
    /// `loginctl lock-session`.
    Lock,
    /// logind `TerminateSession`.
    LogOut,
    Suspend,
    Restart,
    ShutDown,
    /// Re-read settings, apps and pins (the same as `sill reload`).
    Reload,
}
