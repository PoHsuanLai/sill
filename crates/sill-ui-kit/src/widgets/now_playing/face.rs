//! Now Playing's face as data and its one view, shared by the control center's module and the
//! Now Playing widget: the current track as quire's
//! `NowPlayingTrack` (the art from an `IconUrl::file`; a new track cross-fades), Previous,
//! Play or Pause and Next as large toolbar `Button`s, each disabled per `Abilities`, then quire's
//! `TrackPosition` when the player reports both its position and the track's length (design/26
//! 5.2.10, G28-G30). Nothing playing: "Not Playing" and no buttons. The view reads no hook: a
//! press is a [`Transport`] handed to its caller, which sends it to the player.

use ds::IconSwap;
use ds::{Bezel, Button, ControlSize, ImagePosition};
use std::time::Duration;

use dioxus::prelude::*;
use ds::{Availability, ExternalIcon, Icon, IconSize, IconSource, IconUrl, TextLine};
use ds_shell::{NowPlayingTrack, Playback, TrackPosition};
use serde::{Deserialize, Serialize};
use sill_model::now_playing::{Abilities, Art, MediaPlayer, MediaVerb, Micros, Timeline};

/// The Now Playing module's two lines: the title (the player's identity when there is none),
/// and the artists joined with ", " then " — " and the album, leaving out whichever is empty.
pub fn track_lines(player: &MediaPlayer) -> (String, String) {
    let track = &player.track;
    let title = if track.title.is_empty() {
        player.identity.clone()
    } else {
        track.title.clone()
    };
    let artists = track.artists.join(", ");
    let by = [artists.as_str(), track.album.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" — ");
    (title, by)
}

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
    pub playback: sill_model::now_playing::Playback,
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
                    NowPlayingTrack { art: art_of(&track.art), title: track.title.clone(), by }
                }
                div { class: "sill-cc-media-buttons",
                    Button {
                        bezel: Bezel::Toolbar, size: ControlSize::Large, image: ImagePosition::Only,
                        icon: Icon::SkipBack,
                        label: "Previous".to_owned(),
                        availability: available(abilities.previous),
                        onclick: press(Transport::Previous),
                    }
                    Button {
                        bezel: Bezel::Toolbar,
                        size: ControlSize::Large,
                        image: ImagePosition::Only,
                        swap: IconSwap::CrossFade,
                        icon: playback.next_action(),
                        label: playback.label(),
                        availability: available(abilities.play_pause),
                        onclick: press(Transport::PlayPause),
                    }
                    Button {
                        bezel: Bezel::Toolbar, size: ControlSize::Large, image: ImagePosition::Only,
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
pub fn playback_of(playback: sill_model::now_playing::Playback) -> Playback {
    match playback {
        sill_model::now_playing::Playback::Playing => Playback::Playing,
        sill_model::now_playing::Playback::Paused | sill_model::now_playing::Playback::Stopped => {
            Playback::Paused
        }
    }
}

/// The track's times when the player reports both its position and the track's length.
pub fn position_of(timeline: Timeline) -> Option<TrackTimes> {
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
fn available(can: sill_model::units::Availability) -> Availability {
    match can {
        sill_model::units::Availability::Enabled => Availability::Enabled,
        sill_model::units::Availability::Disabled => Availability::Disabled,
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
