//! What a client changes: the brightness and volume keys, the clipboard history, the banners and
//! the keyboard on the newest transient and keycap. The shell's own actions (lock, a screenshot,
//! the dock's pins, the switcher's chord, the workspaces) are `Request::Do`.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};
use sill_launcher::AppId;
use sill_model::intents::IntentsAsk;
use sill_model::keycap::KeycapAsk;

/// A verb that changes something and answers `Done` once it is asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ActRequest {
    /// The brightness and volume keys (`sill display {brightness,volume} ...`), bound to
    /// XF86MonBrightness*/XF86Audio* in the compositor's shortcuts.
    Display(DisplayRequest),
    /// Forget every clipboard entry now (`sill clipboard clear`).
    ClearClipboard,
    /// Dismiss every banner on screen (each sender hears `NotificationClosed`, reason 2).
    DismissNotifications,
    /// Give the keyboard to the newest transient card: the screenshot thumbnail or the newest
    /// banner, whichever appeared last (`sill focus-latest`; bound in the compositor's shortcuts like the
    /// launcher's). Nothing happens when none is showing.
    FocusLatest,
    /// Restart keycap, or end its pause (`sill keycap retry|resume`).
    Keycap(KeycapAsk),
    /// Withdraw a standing terminal grant, or let a paused terminal go on (`sill intents revoke|resume`).
    Intents(IntentsAsk),
    /// Open an installed app the way a click in the dock does (`sill launch <desktop-id>`):
    /// answered `Response::Launched` once the app's process exists, or `Failed`.
    Launch(AppId),
}

/// A display key, or a slider standing in for one (`sill display ...`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(clap::Subcommand))]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DisplayRequest {
    /// The display under the pointer (or every display, per `display.brightness_keys_target`):
    /// a backlight through logind, a monitor over DDC/CI.
    #[cfg_attr(feature = "cli", command(subcommand))]
    Brightness(LevelRequest),
    /// The output volume (and a monitor's own speakers, per `display.hardware_volume`).
    #[cfg_attr(feature = "cli", command(subcommand))]
    Volume(VolumeRequest),
}

/// A level's change: one key press either way, or an absolute percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(clap::Subcommand))]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LevelRequest {
    /// One key press brighter or louder.
    Up,
    /// One key press dimmer or quieter.
    Down,
    /// Straight to PERCENT (0-100).
    Set {
        #[cfg_attr(feature = "cli", arg(value_parser = clap::value_parser!(u8).range(0..=100)))]
        percent: u8,
    },
}

/// A volume change: a level change, or the mute key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(clap::Subcommand))]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum VolumeRequest {
    #[cfg_attr(feature = "cli", command(flatten))]
    Level(LevelRequest),
    Mute,
    Unmute,
    ToggleMute,
}

/// The words of `sill switcher ...`: the window switcher shortcuts run
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

/// The words of `sill workspace ...`: palmrest's swipes run `prev` and `next`.
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
