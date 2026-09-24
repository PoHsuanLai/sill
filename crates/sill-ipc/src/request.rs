//! What a client asks the daemon (plan: `Ping, Launcher{Toggle,Open{query},Close},
//! Dock{Pin,Unpin}, Reload, Quit, Debug{Stats,Pointer,Leave,Capture}`).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// One request. A connection carries exactly one, then its [`crate::Response`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Request {
    /// Is the daemon alive? Answered with `Response::Pong`.
    Ping,
    /// Show, hide or seed the launcher.
    Launcher(LauncherRequest),
    /// Change what the dock keeps.
    Dock(DockRequest),
    /// Re-read settings, the app index and the dock pins without restarting.
    Reload,
    /// Close every surface and exit the daemon.
    Quit,
    /// Development probes (`dev/accept-*.sh`); answered even in a release build, acted on only
    /// when the daemon was built with the `debug` feature.
    Debug(DebugRequest),
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

/// Pin changes (`sill dock {pin,unpin} <desktop-id>`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DockRequest {
    /// Keep `app` in the dock, appended after the last pin.
    Pin { app: DesktopId },
    /// Stop keeping `app`; a running app keeps its tile in the running section.
    Unpin { app: DesktopId },
}

/// Development probes, the IPC form of shell-host's per-surface debug commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DebugRequest {
    /// Frame statistics for every surface since the previous report.
    Stats,
    /// A synthetic pointer motion to `at` on `surface` (surface-local logical px).
    Pointer { surface: SurfaceName, at: Point },
    /// The pointer leaves `surface`.
    Leave { surface: SurfaceName },
    /// Render `surface` offscreen and write a PNG to `path` (compositor-independent capture).
    Capture { surface: SurfaceName, path: PathBuf },
}

/// A desktop entry id without `.desktop`: `org.mozilla.firefox`, `foot`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DesktopId(pub String);

/// The name a debug request addresses a surface by: its layer namespace (`sill-bar-DP-1`,
/// `sill-dock-DP-1`), as shell-host's `SurfaceName` spells it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SurfaceName(pub String);

/// A surface-local point in whole logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
