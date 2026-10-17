//! What the launch service hears and what it can be told.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sill_launcher::{AppId, DesktopAction, OpenTarget};

use super::{FailureCause, LaunchDone, LaunchedScope, ScopeUnit, SettingsPage};

/// What became of a launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LaunchOutcome {
    Started(LaunchedScope),
    Exited(ScopeUnit),
    /// The launch did not happen, or the app closed at once; the person is told (`failure_text`).
    Failed {
        app: AppId,
        cause: FailureCause,
    },
    /// D-Bus activation: no scope of ours to track.
    Activated(AppId),
    /// How one request ended, for the caller that waits to hear.
    Done(LaunchDone),
}

/// One open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenRequest {
    pub target: OpenTarget,
    /// The xdg-activation token the launcher's surface got before it hid.
    pub token: Option<String>,
}

/// One launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub app: AppId,
    pub action: DesktopAction,
    /// Dropped files (`%f`/`%F`).
    pub files: Vec<PathBuf>,
    /// Dropped URIs (`%u`/`%U`).
    pub uris: Vec<String>,
    /// The xdg-activation token for `XDG_ACTIVATION_TOKEN` / `platform-data`; `None` when the
    /// compositor refused one.
    pub token: Option<String>,
}

/// What the launch service can be told.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LaunchCommand {
    Launch(LaunchRequest),
    /// `org.freedesktop.FileManager1.ShowItems([desktop file uri])` (dock Cmd-click).
    ShowInFiles(AppId),
    /// SIGKILL the app's `app-sill-*` scopes (dock Force Quit).
    ForceQuit(AppId),
    /// Open a file, reveal it, or open an address with the default handler (the launcher's rows).
    Open(OpenRequest),
    /// Open the quire Settings app at an account, by bus activation of `org.quire.Settings`.
    OpenSettings(SettingsAccount),
    /// Open the quire Settings app at a page, by bus activation of `org.quire.Settings`.
    OpenSettingsPage(SettingsPage),
}

/// The bus name Settings runs under: the only name porter trusts for the Settings role, so it is
/// always started by activation (as `org.quire.Settings.service`), never as an app scope.
pub const SETTINGS_BUS_NAME: &str = "org.quire.Settings";

/// An account's object path (`/org/quire/Accounts1/account/<segment>`), the place Settings opens at.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SettingsAccount(pub String);
