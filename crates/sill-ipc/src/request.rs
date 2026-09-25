//! What a client asks the daemon (plan: `Ping, Launcher{Toggle,Open{query},Close},
//! Dock{Pin,Unpin}, Reload, Quit, Debug{Stats,Pointer,Leave,Capture}`), plus `Debug(Click)`
//! from the M2 freeze amendment (sill FINDINGS F76), and `Display` from the displays pass (sill
//! FINDINGS "Displays"), `Debug(Osd)` from the OSD, and `PowerMenu`/`ControlCenter` from the
//! M5 freeze (sill FINDINGS "M5 freeze").

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
    /// The brightness and volume keys (`sill display {brightness,volume} ...`), bound to
    /// XF86MonBrightness*/XF86Audio* in COSMIC's shortcuts (`dist/cosmic`).
    Display(DisplayRequest),
    /// Show or hide the power menu (`sill power-menu {open,close,toggle}`).
    PowerMenu(PowerMenuRequest),
    /// Open or close the control center under its bar item (`sill control-center toggle`).
    ControlCenter(ControlCenterRequest),
}

/// The power menu's verbs. It opens on the output the daemon picks (the first with a power menu
/// surface: the output under the pointer is not known to the daemon, FINDINGS F140).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum PowerMenuRequest {
    /// Open, or stay open.
    Open,
    /// Close if open; nothing otherwise.
    Close,
    /// Open when closed, close when open: the power key or a shortcut.
    Toggle,
}

/// The control center's verb: it is a popup of a bar, so it has no open of its own to seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ControlCenterRequest {
    /// Open under the first bar's control center item when closed, close when open.
    Toggle,
}

/// A display key, or a slider standing in for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DisplayRequest {
    /// The display under the pointer (else the focused one), or every display, per
    /// `display.brightness_keys_target`: a backlight through logind, a monitor over DDC/CI.
    Brightness(LevelRequest),
    /// The default sink's volume, and a monitor's own speakers when they are the sink and
    /// `display.hardware_volume` says so.
    Volume(VolumeRequest),
}

/// A level's change: one key press either way, or an absolute percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LevelRequest {
    Up,
    Down,
    /// 0-100; more is taken as 100.
    Set {
        percent: u8,
    },
}

/// A volume change: a level change, or the mute key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum VolumeRequest {
    Level(LevelRequest),
    Mute,
    Unmute,
    ToggleMute,
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
    /// A synthetic press and release of `button` at `at` on `surface` (surface-local logical
    /// px): what opens a bar menu. M2 freeze amendment (F76).
    Click {
        surface: SurfaceName,
        at: Point,
        button: PointerButton,
    },
    /// Show the OSD for a made-up change, on the output a real one of its kind would use,
    /// moving no real level (`dev/accept-osd.sh`: the session's default sink may be somebody's
    /// headphones).
    Osd(FakeOsd),
}

/// A made-up OSD change: what moved, to which level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FakeOsd {
    pub topic: OsdTopic,
    /// 0-100; more is taken as 100.
    pub percent: u8,
    pub mute: FakeMute,
}

/// What an OSD change is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsdTopic {
    Volume,
    Brightness,
}

/// Whether a made-up volume change is muted (a brightness change never is).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FakeMute {
    #[default]
    Audible,
    Muted,
}

/// Which pointer button a debug click presses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerButton {
    /// The left button of a right-handed mouse.
    #[default]
    Primary,
    /// The right button: context menus.
    Secondary,
    /// The wheel button.
    Middle,
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
