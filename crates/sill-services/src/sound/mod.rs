//! UI sounds from the freedesktop sound theme through libpulse, samples uploaded once and
//! played with `play_sample` (design/13 §13.3.10). Gated by `sound.ui_sounds` and
//! `sound.volume_feedback`, which the caller checks.

#![allow(unused_variables, dead_code)] // Freeze stubs

use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;
use crate::units::Presence;

/// A freedesktop sound-naming-spec event name: `trash-empty`, `item-deleted`, `bell`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SoundName(pub String);

/// What the sound service can be told.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum SoundCommand {
    Play(SoundName),
    /// Use this theme (`sound.theme`) from now on.
    Theme(String),
}

/// Start the sound thread; the receiver says whether a pulse server answered.
pub fn spawn(rt: &Handle) -> (watch::Receiver<Presence>, Control<SoundCommand>) {
    todo!("M6: libpulse-binding upload_sample per theme file, play_sample")
}
