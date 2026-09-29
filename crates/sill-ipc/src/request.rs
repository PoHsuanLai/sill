//! What a client asks the daemon: one [`Request`] per connection. The desktop's words live in
//! `desktop.rs`, the session's (switcher, lock, polkit) in `session.rs`, the launcher's in
//! `launcher.rs` and idle's in `idle.rs`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::desktop::{FakeCorner, FakeScreenshot, ScreenshotRequest, WidgetsRequest};
use crate::idle::IdleRequest;
use crate::launcher::{ClipboardRequest, FakeLauncherKey};
use crate::session::{
    FakeAuth, FakePolkit, FakeSwitcher, LockRequest, PolkitRequest, SwitcherRequest,
};

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
    /// Re-read the settings without restarting.
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
    /// The notification center and the banners (`sill notifications ...`).
    Notifications(NotificationsRequest),
    /// Lock the screen now (`sill lock`; COSMIC's `LockScreen` system action in `dist/cosmic`),
    /// answered `Done` once asked; or say what the lock screen is doing (`sill lock status`).
    Lock(LockRequest),
    /// Take a screenshot with `screenshot.tool` (`sill screenshot`).
    Screenshot(ScreenshotRequest),
    /// The widgets (`sill widgets ...`).
    Widgets(WidgetsRequest),
    /// The app switcher's chord verbs (`sill switcher ...`).
    Switcher(SwitcherRequest),
    /// sill's polkit agent (`sill polkit status`).
    Polkit(PolkitRequest),
    /// The clipboard history (`sill clipboard status|clear`).
    Clipboard(ClipboardRequest),
    /// sill's own idle (`sill idle status`).
    Idle(IdleRequest),
}

/// `sill notifications ...`. The center opens on the output the daemon picks (the first with a
/// center surface), as the power menu does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum NotificationsRequest {
    /// Open the notification center when closed, close it when open: a shortcut.
    Toggle,
    /// Open it, or keep it open.
    Open,
    /// Close it if open; nothing otherwise.
    Close,
    /// Dismiss every banner on screen (each sender hears `NotificationClosed`, reason 2).
    DismissAll,
    /// Who holds `org.freedesktop.Notifications`, and how many banners and kept notifications
    /// there are: answered with `Response::Notifications`.
    Status,
}

/// The power menu's verbs. It opens on the output the daemon picks (the first with a power menu
/// surface: the output under the pointer is not known to the daemon).
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
    /// A synthetic press and release of `button` at `at` on `surface` (surface-local logical px):
    /// what opens a bar menu.
    Click {
        surface: SurfaceName,
        at: Point,
        button: PointerButton,
    },
    /// `button` presses at `at` on `surface` and stays down: the first half of a real drag
    /// (shell-host's `Inject::PointerDown`). The existing `Pointer` moves while it is held
    /// (shell-host now carries the held buttons on every move, the same `PointerMove` a real
    /// `Motion` produces); `PointerRelease` lets go.
    PointerPress {
        surface: SurfaceName,
        at: Point,
        button: PointerButton,
    },
    /// `button` releases at `at` on `surface`, the second half of a real drag
    /// (`Inject::PointerUp`).
    PointerRelease {
        surface: SurfaceName,
        at: Point,
        button: PointerButton,
    },
    /// Show the OSD for a made-up change, on the output a real one of its kind would use,
    /// moving no real level (`dev/accept-osd.sh`: the session's default sink may be somebody's
    /// headphones).
    Osd(FakeOsd),
    /// Post a made-up notification straight to the notifications service, without D-Bus
    /// (`dev/accept-notifications.sh`): it goes through the same reducer as a `Notify`, so it
    /// shows a banner and enters the history, and its actions answer nobody.
    Notify(FakeNotification),
    /// Show the screenshot thumbnail for a file, as if the folder watch had seen it
    /// (`dev/accept-screenshot.sh`: nothing of the session is captured).
    Screenshot(FakeScreenshot),
    /// Carry out a hot corner's action as if its dwell had run out (`dev/accept-hot-corners.sh`).
    Corner(FakeCorner),
    /// Feed the lock screen's password check a made-up verdict; refused by a daemon built
    /// without `debug` (`dev/accept-lock.sh`, on the nested compositor only).
    Auth(FakeAuth),
    /// Show a made-up polkit request (`dev/accept-polkit.sh`).
    Polkit(FakePolkit),
    /// Open the switcher on made-up apps (`dev/accept-switcher.sh`).
    Switcher(FakeSwitcher),
    /// A key for the open launcher, as if typed (`dev/accept-launcher-v2.sh`).
    LauncherKey(FakeLauncherKey),
    /// The open launcher's sections, rows and preview: answered `Response::LauncherRows`.
    LauncherRows,
    /// Wake idle as the compositor's `resumed` would after real input (`dev/accept-idle.sh`:
    /// the nested cosmic-comp counts no synthetic input as activity). Refused by a daemon built
    /// without `debug`: inside the lock grace it opens the lock.
    IdleWake,
}

/// A made-up notification: the parts of `Notify` an acceptance needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FakeNotification {
    /// `app_name` (and the group it falls in).
    pub app: String,
    pub summary: String,
    /// Markup, as a sender would write it.
    pub body: String,
    pub urgency: FakeUrgency,
    /// Buttons, in order; `default` is the body's click.
    pub actions: Vec<FakeAction>,
    pub expire: FakeExpire,
    /// The id it replaces (0: none).
    pub replaces: u32,
}

/// A made-up notification's urgency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FakeUrgency {
    Low,
    #[default]
    Normal,
    Critical,
}

/// One made-up action: its key and its button's label.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FakeAction {
    pub key: String,
    pub label: String,
}

/// A made-up notification's `expire_timeout`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FakeExpire {
    /// -1: the banner's usual hold.
    #[default]
    Default,
    /// 0: an alert that stays until acted on.
    Never,
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
