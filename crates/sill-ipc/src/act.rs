//! What a client changes: the dock's pins, the brightness and volume keys, the lock, a
//! screenshot, the desktop widgets, the switcher's chord, the clipboard history, the banners and
//! the keyboard on the newest transient and keycap.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};
use sill_launcher::AppId;
use sill_model::intents::IntentsAsk;
use sill_model::keycap::KeycapAsk;
use sill_model::screenshots::ShotKind;

/// A verb that changes something and answers `Done` once it is asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ActRequest {
    /// Change what the dock keeps (`sill dock {pin,unpin} <desktop-id>`).
    Dock(DockRequest),
    /// The brightness and volume keys (`sill display {brightness,volume} ...`), bound to
    /// XF86MonBrightness*/XF86Audio* in the compositor's shortcuts.
    Display(DisplayRequest),
    /// Lock the screen now (`sill lock`).
    Lock,
    /// Take a screenshot with `screenshot.tool` (`sill screenshot [area|window|screen]`): the
    /// file it saves shows as the thumbnail through the folder watch.
    Screenshot(ShotKind),
    /// Flip `widgets.desktop` in `settings.toml` (the file stays the one truth; the daemon's
    /// watch picks the change up).
    ToggleDesktopWidgets,
    /// The app switcher's chord verbs (`sill switcher ...`).
    Switcher(SwitcherRequest),
    /// Forget every clipboard entry now (`sill clipboard clear`).
    ClearClipboard,
    /// Dismiss every banner on screen (each sender hears `NotificationClosed`, reason 2).
    DismissNotifications,
    /// Give the keyboard to the newest transient card: the screenshot thumbnail or the newest
    /// banner, whichever appeared last (`sill focus-latest`; bound in the compositor's shortcuts like the
    /// launcher's). Nothing happens when none is showing.
    FocusLatest,
    /// Step the workspace, or open the overview (`sill workspace {next,prev,overview}`; palmrest's
    /// two-finger swipes run these).
    Workspace(WorkspaceRequest),
    /// Restart keycap, or end its pause (`sill keycap retry|resume`).
    Keycap(KeycapAsk),
    /// Withdraw a standing terminal grant, or let a paused terminal go on (`sill intents revoke|resume`).
    Intents(IntentsAsk),
    /// Open an installed app the way a click in the dock does (`sill launch <desktop-id>`):
    /// answered `Response::Launched` once the app's process exists, or `Failed`.
    Launch(AppId),
}

/// Pin changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DockRequest {
    /// Keep `app` in the dock, appended after the last pin.
    Pin { app: AppId },
    /// Stop keeping `app`; a running app keeps its tile in the running section.
    Unpin { app: AppId },
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

/// `sill switcher ...`: the window switcher shortcuts run
/// `next` / `prev`; `cancel` and `commit` are for scripts and for a compositor
/// that tells sill about the modifier's release some other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum SwitcherRequest {
    /// The chord (Cmd+Tab): arm the switcher, or move the selection on.
    Next,
    /// The reverse chord (Cmd+Shift+Tab).
    Prev,
    /// Close it and switch to nothing (Escape).
    Cancel,
    /// Switch to the selection and close it (the modifier's release).
    Commit,
}

/// `sill workspace ...`: palmrest's swipes run `prev` and `next`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRequest {
    /// The workspace before the active one (none at the first).
    Prev,
    /// The workspace after the active one (none at the last).
    Next,
    /// Mission Control, on the desktop's own compositor; any other answers `Unsupported`.
    Overview,
}
