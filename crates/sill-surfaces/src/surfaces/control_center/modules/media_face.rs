//! Now Playing's face as data and its one view, shared by the control center's module and the
//! Now Playing widget (`crate::widgets::now_playing`): the current track as quire's
//! `NowPlayingTrack` (the art from an `IconUrl::file`; a new track cross-fades), Previous,
//! quire's `PlayPauseButton` and Next, each disabled per `Abilities`, then quire's
//! `TrackPosition` when the player reports both its position and the track's length (design/26
//! 5.2.10, G28-G30). Nothing playing: "Not Playing" and no buttons. The view reads no hook: a
//! press is a [`Transport`] handed to its caller, which sends it to the player.

use std::time::Duration;

use dioxus::prelude::*;
use ds::{
    Availability, ExternalIcon, Icon, IconButton, IconButtonVariant, IconSize, IconSource, IconUrl,
    NowPlayingTrack, PlayPauseButton, Playback, TextLine, TrackPosition,
};
use serde::{Deserialize, Serialize};
use sill_services::{Abilities, Art, MediaPlayer, MediaVerb, Micros, Timeline};

use crate::surfaces::control_center::track_lines;

/// What Now Playing shows.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaFace {
    /// No player: "Not Playing".
    #[default]
    Idle,
    /// The current player's track.
    Track(TrackFace),
}

/// The current track, its transport and where it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackFace {
    /// The title (the player's identity when there is none).
    pub title: String,
    /// The artists and the album; empty when neither is known.
    pub by: String,
    /// The cover.
    pub art: Art,
    /// Playing, paused or stopped.
    pub playback: sill_services::Playback,
    /// Which buttons work.
    pub abilities: Abilities,
    /// Where the track is and how long it is, when both are known.
    pub times: Option<TrackTimes>,
}

/// Where the track is and how long it is, as `TrackPosition` takes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackTimes {
    pub at: Duration,
    pub length: Duration,
}

/// A transport button's press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Previous,
    PlayPause,
    Next,
}

impl Transport {
    /// The service's verb.
    pub fn verb(self) -> MediaVerb {
        match self {
            Transport::Previous => MediaVerb::Previous,
            Transport::PlayPause => MediaVerb::PlayPause,
            Transport::Next => MediaVerb::Next,
        }
    }
}

/// `player`'s face, or `Idle` without one.
pub fn media_face(player: Option<&MediaPlayer>) -> MediaFace {
    let Some(player) = player else {
        return MediaFace::Idle;
    };
    let (title, by) = track_lines(player);
    MediaFace::Track(TrackFace {
        title,
        by,
        art: player.track.art.clone(),
        playback: player.playback,
        abilities: player.abilities,
        times: position_of(player.timeline),
    })
}

/// The face, its presses sent to `onpress` (none draws the buttons inert).
#[component]
pub fn MediaView(face: MediaFace, onpress: Option<EventHandler<Transport>>) -> Element {
    let MediaFace::Track(track) = face else {
        return rsx! {
            div { class: "sill-cc-media",
                NowPlayingTrack { art: None::<IconSource>, title: "Not Playing" }
            }
        };
    };
    let press = move |transport: Transport| {
        move |_| {
            if let Some(onpress) = onpress {
                onpress.call(transport);
            }
        }
    };
    let playback = playback_of(track.playback);
    let abilities = track.abilities;
    let by = (!track.by.is_empty()).then(|| TextLine::from(track.by.clone()));
    rsx! {
        div { class: "sill-cc-media",
            div { class: "sill-cc-media-row",
                div { class: "sill-cc-media-track",
                    NowPlayingTrack { art: art_of(&track.art), title: track.title.clone(), by, playback }
                }
                div { class: "sill-cc-media-buttons",
                    IconButton {
                        variant: IconButtonVariant::Tool,
                        icon: Icon::SkipBack,
                        label: "Previous".to_owned(),
                        availability: available(abilities.previous),
                        onclick: press(Transport::Previous),
                    }
                    PlayPauseButton {
                        playback,
                        availability: available(abilities.play_pause),
                        onclick: press(Transport::PlayPause),
                    }
                    IconButton {
                        variant: IconButtonVariant::Tool,
                        icon: Icon::SkipForward,
                        label: "Next".to_owned(),
                        availability: available(abilities.next),
                        onclick: press(Transport::Next),
                    }
                }
            }
            if let Some(TrackTimes { at, length }) = track.times {
                TrackPosition { at, length, playback }
            }
        }
    }
}

/// The service's playback as quire's: a stopped player offers Play, as a paused one does.
pub(crate) fn playback_of(playback: sill_services::Playback) -> Playback {
    match playback {
        sill_services::Playback::Playing => Playback::Playing,
        sill_services::Playback::Paused | sill_services::Playback::Stopped => Playback::Paused,
    }
}

/// The track's times when the player reports both its position and the track's length.
pub(crate) fn position_of(timeline: Timeline) -> Option<TrackTimes> {
    match timeline {
        Timeline::Known {
            position, length, ..
        } => Some(TrackTimes {
            at: duration_of(position),
            length: duration_of(length),
        }),
        Timeline::Unknown => None,
    }
}

fn duration_of(micros: Micros) -> Duration {
    Duration::from_micros(micros.0)
}

/// The service's word for "can do it" as quire's.
fn available(can: sill_services::Availability) -> Availability {
    match can {
        sill_services::Availability::Enabled => Availability::Enabled,
        sill_services::Availability::Disabled => Availability::Disabled,
    }
}

/// The track's art at 48, or none (quire draws the Now Playing glyph on its well).
fn art_of(art: &Art) -> Option<IconSource> {
    match art {
        Art::File(path) => IconUrl::file(path).ok().map(|url| {
            IconSource::Image(ExternalIcon {
                url,
                size: IconSize::Tile48,
            })
        }),
        Art::None => None,
    }
}
