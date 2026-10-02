//! Every MPRIS player's state and which one is current.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::units::Availability;
use ds_core::time::stamp::Stamp;

/// Every MPRIS player on the bus, in the order they appeared.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Media {
    pub players: Vec<MediaPlayer>,
}

/// A player's bus name (`org.mpris.MediaPlayer2.spotify`, `...firefox.instance_1_42`): its
/// identity for as long as it is on the bus.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerId(pub String);

/// One player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaPlayer {
    pub id: PlayerId,
    /// `org.mpris.MediaPlayer2.Identity` ("Spotify", "Mozilla Firefox").
    pub identity: String,
    /// `PlaybackStatus`.
    pub playback: Playback,
    /// When `playback` last changed: the most recent player to start playing is the one shown.
    pub changed: Stamp,
    /// `Metadata`, the parts the module shows.
    pub track: Track,
    pub timeline: Timeline,
    pub abilities: Abilities,
}

/// `PlaybackStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Playback {
    Playing,
    Paused,
    #[default]
    Stopped,
}

/// What is playing.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Track {
    /// `xesam:title`; empty when the player gives none.
    pub title: String,
    /// `xesam:artist`, in order.
    pub artists: Vec<String>,
    /// `xesam:album`; empty when the player gives none.
    pub album: String,
    pub art: Art,
}

/// `mpris:artUrl`, once it is a local file the surface can load.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Art {
    /// No art, or remote art not fetched (yet, or ever).
    #[default]
    None,
    /// An absolute path: the player's own `file://` art, or the cached download of remote art.
    File(PathBuf),
}

/// Microseconds: MPRIS's unit for `Position` and `mpris:length`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Micros(pub u64);

/// Where in the track the player is, when it says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Timeline {
    /// No `mpris:length`, or no `Position` (a stream).
    #[default]
    Unknown,
    /// `Position` read (or `Seeked`) at `at`; a playing track's position now is
    /// `position + (now - at)`, capped at `length`.
    Known {
        position: Micros,
        length: Micros,
        at: Stamp,
    },
}

/// Which transport buttons the player takes: `CanPlay`/`CanPause`, `CanGoNext`,
/// `CanGoPrevious` (each `Disabled` also when `CanControl` is false).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Abilities {
    pub play_pause: Availability,
    pub next: Availability,
    pub previous: Availability,
}

impl Media {
    /// The player the control center shows: the one that most recently started playing; with
    /// none playing, the most recently paused; a stopped player is never shown.
    pub fn current(&self) -> Option<&MediaPlayer> {
        self.players
            .iter()
            .filter(|p| p.playback == Playback::Playing)
            .max_by_key(|p| p.changed)
            .or_else(|| {
                self.players
                    .iter()
                    .filter(|p| p.playback == Playback::Paused)
                    .max_by_key(|p| p.changed)
            })
    }
}
