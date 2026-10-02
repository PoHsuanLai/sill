//! What the now playing service hears and what it can be told.

use serde::{Deserialize, Serialize};

use super::{Abilities, MediaPlayer, Playback, PlayerId, Timeline, Track};
use ds_core::time::stamp::Stamp;

/// One change on the bus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum MediaEvent {
    /// A player appeared (every property read), replacing one with its id.
    Player(MediaPlayer),
    /// `PlaybackStatus` changed at `at`.
    Playback {
        id: PlayerId,
        playback: Playback,
        at: Stamp,
    },
    Track {
        id: PlayerId,
        track: Track,
    },
    Timeline {
        id: PlayerId,
        timeline: Timeline,
    },
    Abilities {
        id: PlayerId,
        abilities: Abilities,
    },
    /// The player's name left the bus.
    Removed(PlayerId),
    /// The bus is gone.
    Gone,
}

/// One transport button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaVerb {
    PlayPause,
    Next,
    Previous,
}

/// What the Now Playing service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaCommand {
    /// Press `verb` on `player` (the one the surface showed, so a player that took over in
    /// between is not the one that pauses). A verb the player cannot do is dropped.
    Press { player: PlayerId, verb: MediaVerb },
    /// Re-read every player (after a resume).
    Refresh,
    /// Re-read `player`'s `Position` (and the track's `mpris:length`) and republish its
    /// timeline: a read, nothing sent to the player. MPRIS signals no position change as a
    /// track plays, so a surface that opens on a player asks for where it is now.
    Position(PlayerId),
}
