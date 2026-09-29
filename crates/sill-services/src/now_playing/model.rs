//! Every MPRIS player's state, which one is current, and the one reducer.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use sill_model::units::{Availability, Stamp};

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

/// `media` after `event`. A change for an id that is not listed is dropped.
pub fn step(mut media: Media, event: MediaEvent) -> Media {
    match event {
        MediaEvent::Player(player) => {
            match media.players.iter_mut().find(|p| p.id == player.id) {
                Some(existing) => *existing = player,
                None => media.players.push(player),
            }
            media
        }
        MediaEvent::Playback { id, playback, at } => {
            if let Some(player) = media.players.iter_mut().find(|p| p.id == id) {
                player.playback = playback;
                player.changed = at;
            }
            media
        }
        MediaEvent::Track { id, track } => {
            if let Some(player) = media.players.iter_mut().find(|p| p.id == id) {
                player.track = track;
            }
            media
        }
        MediaEvent::Timeline { id, timeline } => {
            if let Some(player) = media.players.iter_mut().find(|p| p.id == id) {
                player.timeline = timeline;
            }
            media
        }
        MediaEvent::Abilities { id, abilities } => {
            if let Some(player) = media.players.iter_mut().find(|p| p.id == id) {
                player.abilities = abilities;
            }
            media
        }
        MediaEvent::Removed(id) => {
            media.players.retain(|p| p.id != id);
            media
        }
        MediaEvent::Gone => Media::default(),
    }
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

#[cfg(test)]
mod tests {
    use super::{
        Abilities, Media, MediaEvent, MediaPlayer, Playback, PlayerId, Timeline, Track, step,
    };
    use sill_model::units::Stamp;

    fn player(name: &str, playback: Playback, changed: u64) -> MediaPlayer {
        MediaPlayer {
            id: PlayerId(format!("org.mpris.MediaPlayer2.{name}")),
            identity: name.to_owned(),
            playback,
            changed: Stamp(changed),
            track: Track::default(),
            timeline: Timeline::Unknown,
            abilities: Abilities::default(),
        }
    }

    fn current(players: Vec<MediaPlayer>) -> Option<String> {
        Media { players }.current().map(|p| p.identity.clone())
    }

    #[test]
    fn the_most_recent_player_to_play_is_current() {
        use Playback::{Paused, Playing, Stopped};
        let cases: Vec<(&str, Vec<MediaPlayer>, Option<&str>)> = vec![
            ("nothing on the bus", Vec::new(), None),
            ("stopped only", vec![player("a", Stopped, 9)], None),
            (
                "playing beats a later pause",
                vec![player("a", Playing, 1), player("b", Paused, 9)],
                Some("a"),
            ),
            (
                "the later of two playing",
                vec![player("a", Playing, 1), player("b", Playing, 5)],
                Some("b"),
            ),
            (
                "the later of two paused",
                vec![player("a", Paused, 7), player("b", Paused, 5)],
                Some("a"),
            ),
        ];
        for (name, players, want) in cases {
            assert_eq!(current(players), want.map(str::to_owned), "{name}");
        }
    }

    #[test]
    fn events_reduce_to_the_expected_media() {
        let a = || player("a", Playback::Paused, 1);
        let with = |players| Media { players };
        let cases: Vec<(&str, Media, MediaEvent, Media)> = vec![
            (
                "a player appears",
                Media::default(),
                MediaEvent::Player(a()),
                with(vec![a()]),
            ),
            (
                "it starts playing",
                with(vec![a()]),
                MediaEvent::Playback {
                    id: a().id,
                    playback: Playback::Playing,
                    at: Stamp(4),
                },
                with(vec![player("a", Playback::Playing, 4)]),
            ),
            (
                "an unknown player's change is dropped",
                with(vec![a()]),
                MediaEvent::Playback {
                    id: PlayerId("nobody".to_owned()),
                    playback: Playback::Playing,
                    at: Stamp(4),
                },
                with(vec![a()]),
            ),
            (
                "it leaves",
                with(vec![a()]),
                MediaEvent::Removed(a().id),
                Media::default(),
            ),
            (
                "the bus goes",
                with(vec![a()]),
                MediaEvent::Gone,
                Media::default(),
            ),
        ];
        for (name, before, event, want) in cases {
            assert_eq!(step(before, event), want, "{name}");
        }
    }
}
