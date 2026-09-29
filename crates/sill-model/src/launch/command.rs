//! What the launch service hears and what it can be told.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sill_launcher::{AppId, DesktopAction, OpenTarget};

use super::{LaunchedScope, ScopeUnit};

/// What became of a launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LaunchOutcome {
    Started(LaunchedScope),
    Exited(ScopeUnit),
    /// The launch did not happen; the dock stops its bounce.
    Failed {
        app: AppId,
        reason: String,
    },
    /// D-Bus activation: no scope of ours to track.
    Activated(AppId),
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
}
