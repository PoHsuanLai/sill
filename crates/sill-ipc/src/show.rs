//! The words of the panels a client opens and closes: how a panel changes, the launcher's seeded
//! open and close, the companion's summon and hold-to-talk. Each panel opens on the output the
//! daemon picks (the output under the pointer is not known to the daemon), which is the first with
//! that surface.

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

/// How a panel changes: the shortcut that flips it, or the two that set it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
pub enum Reveal {
    /// Open when closed, close when open.
    Toggle,
    /// Open, or stay open.
    Open,
    /// Close if open; nothing otherwise.
    Close,
}

/// The launcher's two verbs that are not an action (`sill launcher {open [--query],close}`;
/// `toggle` is the `launcher` action).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LauncherRequest {
    /// Open with `query` already typed; an empty query opens a fresh launcher.
    Open { query: String },
    /// Close if open; nothing otherwise.
    Close,
}

/// The hold-to-talk gesture's three edges, from a source that detects the hold itself (the
/// keyboard remapper): the key held long enough, let go, or spoiled by another input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
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
