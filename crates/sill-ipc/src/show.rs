//! The panels a client opens and closes: the launcher, the power menu, the control center, the
//! notification center, Edit Widgets and the companion. Each opens on the output the daemon picks (the output
//! under the pointer is not known to the daemon), which is the first with that surface; Edit
//! Widgets prefers the output of the active window.

use serde::{Deserialize, Serialize};

/// `sill launcher|power-menu|control-center|notifications|widgets ...`, the verbs that show or
/// hide something.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ShowRequest {
    /// Show, hide or seed the launcher.
    Launcher(LauncherRequest),
    /// The power menu (`sill power-menu {open,close,toggle}`).
    PowerMenu(Reveal),
    /// The notification center (`sill notifications {toggle,open,close}`).
    NotificationCenter(Reveal),
    /// The control center under the first bar's item: it is a popup of a bar, so it has no open
    /// of its own to seed. Opens when closed, closes when open.
    ControlCenter,
    /// Edit Widgets, the widget gallery (`sill widgets {edit,edit-close}`).
    EditWidgets(EditWidgetsRequest),
    /// The companion's summon (`sill companion {tap,summon,dismiss,stop-all}`).
    Companion(CompanionRequest),
    /// Hold-to-talk (`sill voice {hold-start,hold-end,hold-cancel}`).
    Voice(VoiceRequest),
}

/// How a panel changes: the shortcut that flips it, or the two that set it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reveal {
    /// Open when closed, close when open.
    Toggle,
    /// Open, or stay open.
    Open,
    /// Close if open; nothing otherwise.
    Close,
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

/// Edit Widgets has no toggle: it opens on the output of the active window when that output has
/// its surface, else on the first that has, and closes wherever it is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditWidgetsRequest {
    Open,
    Close,
}

/// The hold-to-talk gesture's three edges, from a source that detects the hold itself (the
/// keyboard remapper): the key held long enough, let go, or spoiled by another input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceRequest {
    /// The hold began: open the companion and listen.
    HoldStart,
    /// The key was let go: finish and send what was said.
    HoldEnd,
    /// Another key or the pointer spoiled the hold: discard what was said.
    HoldCancel,
}

/// The companion's four verbs (`sill companion {tap,summon,dismiss,stop-all}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionRequest {
    /// The double-tap fired as a single tap of the key (the keyboard remapper's route B): the
    /// daemon's `Tap` machine decides whether this one is the second.
    Tap,
    /// A double-tap already detected (route A, or the compositor later).
    Summon,
    /// Put the companion away: close the launcher it opened, or end the prompt in an app.
    Dismiss,
    /// Pause every run in the focused Space (the soft form of the kill switch; the hard chord is
    /// the compositor's).
    StopAll,
}
