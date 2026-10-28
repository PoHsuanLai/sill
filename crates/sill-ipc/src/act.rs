//! The words of the keys and chords a client sends: the brightness and volume keys (`Request::Display`),
//! and the switcher's and the workspaces' words, which the command line turns into actions. The
//! shell's own actions (lock, a screenshot, the dock's pins, the switcher's chord, the workspaces)
//! are `Request::Do`.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

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
