//! Development probes, the IPC form of shell-host's per-surface debug commands and of the
//! made-up inputs an acceptance script drives the daemon with. A made-up input is the real
//! `sill-model` value the service would have received (a `Posting`, a `FakeRequest`, a
//! `FakeVerdict`). The names of surfaces, points and buttons are this wire's own words for
//! shell-host's: this crate may not depend on shell-host (`scripts/check-boundary.sh`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sill_launcher::{AppId, PreviewKind};
use sill_model::auth::FakeVerdict;
use sill_model::notifications::Posting;
use sill_model::polkit_agent::FakeRequest;
use sill_model::units::Percent;
use sill_model::volume::Mute;

/// A probe. Every one is answered even in a release build; the ones that act are refused there.
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
    /// (shell-host's `Inject::PointerDown`). `Pointer` moves while it is held; `PointerRelease`
    /// lets go.
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
    /// A key on `surface`, as if typed: it enters the surface where a real key does
    /// (shell-host's `Inject::Key`), so the surface reads it whether or not it holds the
    /// compositor's keyboard focus.
    Key {
        surface: SurfaceName,
        key: KeySpec,
        action: KeyAction,
    },
    /// Show the OSD for a made-up change, on the output a real one of its kind would use,
    /// moving no real level (`dev/accept-osd.sh`: the session's default sink may be somebody's
    /// headphones). `percent` is 0-100; more is taken as 100.
    Osd {
        topic: OsdTopic,
        percent: Percent,
        mute: Mute,
    },
    /// Post a made-up notification straight to the notifications service, without D-Bus
    /// (`dev/accept-notifications.sh`): it goes through the same reducer as a `Notify`, so it
    /// shows a banner and enters the history, and its actions answer nobody.
    Notify(Box<Posting>),
    /// Show the screenshot thumbnail for a file, as if the folder watch had seen it
    /// (`dev/accept-screenshot.sh`: nothing of the session is captured).
    Screenshot { path: PathBuf },
    /// Carry out a hot corner's action on the first output, as if its dwell had just run out
    /// (`dev/accept-hot-corners.sh`).
    Corner(CornerName),
    /// Feed the lock screen's password check a made-up verdict; refused by a daemon built
    /// without `debug` (`dev/accept-lock.sh`, on the nested compositor only).
    Auth(FakeVerdict),
    /// Show a made-up polkit request (`dev/accept-polkit.sh`); the stand-in helper accepts
    /// `sill-fake`.
    Polkit(FakeRequest),
    /// Open the switcher on made-up apps, the current one first (`dev/accept-switcher.sh`): shown
    /// at once (no show delay), and Q, H and release act on no real window.
    Switcher { apps: Vec<AppId> },
    /// A key for the open launcher, as if typed (`dev/accept-launcher-v2.sh`).
    LauncherKey(LauncherKey),
    /// The open launcher's sections, rows and preview: answered `Response::LauncherRows`.
    LauncherRows,
    /// Wake idle as the compositor's `resumed` would after real input (`dev/accept-idle.sh`:
    /// the nested cosmic-comp counts no synthetic input as activity). Refused by a daemon built
    /// without `debug`: inside the lock grace it opens the lock.
    IdleWake,
}

/// What an OSD change is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsdTopic {
    Volume,
    Brightness,
}

/// A screen corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CornerName {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
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

/// The name a debug request addresses a surface by: its layer namespace (`sill-bar-DP-1`,
/// `sill-dock-DP-1`), as shell-host's `SurfaceName` spells it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SurfaceName(pub String);

/// A key in shell-host's spelling: a W3C key name or `Space`, then `+shift`, `+ctrl`, `+alt`
/// or `+super` (`Tab`, `ArrowDown`, `k+ctrl`, `Tab+shift`). Parsed by the daemon, which answers a
/// failure for a name it does not know.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeySpec(pub String);

/// What an injected key does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyAction {
    /// Press and release: a keystroke.
    #[default]
    Tap,
    /// Press and hold.
    Press,
    /// Let go of a held key.
    Release,
}

/// A surface-local point in whole logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// A key for the launcher, as if typed (`sill debug launcher-key`). Acted on only by a daemon
/// built with `debug`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LauncherKey {
    Up,
    Down,
    Enter,
    Escape,
    Tab,
    ShiftTab,
    /// Ctrl+K.
    Actions,
    /// Space: the preview pane (Quick Look).
    Space,
    /// Ctrl+Y (Cmd+Y): the preview pane from anywhere.
    QuickLook,
    Right,
    Left,
    /// Ctrl+R (Cmd+R): the selected row's reveal.
    Reveal,
}

/// The launcher as shown on the output it is open on (`sill debug launcher-rows`): each section
/// with its row titles, the selection, and the preview pane. Answered only by a daemon built
/// with `debug`; clipboard rows' titles are replaced by `"<clipboard>"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LauncherRows {
    pub query: String,
    pub sections: Vec<RowsSection>,
    /// The selected row, counted over every section's rows.
    pub selected: u32,
    /// The preview pane's kind while it is shown; `None` while it is hidden.
    pub preview: Option<PreviewKind>,
}

/// One section as shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowsSection {
    pub title: String,
    pub rows: Vec<String>,
    /// Rows hidden behind "Show More".
    pub hidden: u32,
}

/// One surface's frame statistics, the wire's form of shell-host's `FrameStats`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceStats {
    pub surface: SurfaceName,
    /// Frames presented since the surface was created.
    pub frames: u64,
    /// Frame callbacks received.
    pub callbacks: u64,
    /// `wl_surface.commit`s; idle means this number stops moving.
    pub commits: u64,
    /// Frames that were due but not presented.
    pub skipped: u64,
    /// Watchdog expiries.
    pub watchdog_fired: u64,
    /// Trigger-to-present time over the samples since the previous report.
    pub work: Micros,
    /// Present-to-present time over the same samples.
    pub interval: Micros,
    /// How many frames the two percentile sets cover.
    pub samples: u32,
    /// The daemon's resident set size, KiB.
    pub vm_rss_kib: u64,
}

/// Nearest-rank percentiles of a set of durations, in microseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Micros {
    pub p50_us: u32,
    pub p95_us: u32,
    pub max_us: u32,
}
