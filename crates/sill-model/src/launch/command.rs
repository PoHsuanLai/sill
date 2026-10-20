//! What the launch service hears and what it can be told.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sill_launcher::{AppId, DesktopAction, OpenTarget};

use super::{FailureCause, LaunchDone, LaunchedScope, ScopeUnit, SettingsOpen};

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

impl LaunchRequest {
    /// A plain start of `app`: its own `Exec`, no files, no addresses, no token. The one place
    /// a request is spelled out; every launch builds on it.
    pub fn app(app: AppId) -> LaunchRequest {
        LaunchRequest {
            app,
            action: DesktopAction::Default,
            files: Vec::new(),
            uris: Vec::new(),
            token: None,
        }
    }

    /// This request starting the desktop action `action` instead of the entry's own `Exec`.
    pub fn action(self, action: DesktopAction) -> LaunchRequest {
        LaunchRequest { action, ..self }
    }

    /// This request handing over `uris` (`%u`/`%U`).
    pub fn uris(self, uris: Vec<String>) -> LaunchRequest {
        LaunchRequest { uris, ..self }
    }

    /// This request carrying `token`.
    pub fn token(self, token: Option<String>) -> LaunchRequest {
        LaunchRequest { token, ..self }
    }
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
    /// Open the quire Settings app at a place, by bus activation of `org.quire.Settings`.
    OpenSettings(SettingsOpen),
}

/// The bus name Settings runs under: the only name porter trusts for the Settings role, so it is
/// always started by activation (as `org.quire.Settings.service`; its desktop entry is
/// `DBusActivatable`), never as an app scope.
pub const SETTINGS_BUS_NAME: &str = "org.quire.Settings";
