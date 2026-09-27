//! The Now Playing module: `Media::current`'s track as quire's `NowPlayingTrack` (the art from
//! an `IconUrl::file`, [`crate::surfaces::control_center::track_lines`]; a new track
//! cross-fades), Previous, quire's `PlayPauseButton` and Next, each disabled per `Abilities` and
//! sending `MediaCommand::Press`, then quire's `TrackPosition` when the player reports both its
//! position and the track's length (design/26 5.2.10, G28-G30). Nothing playing: the module
//! shows "Not Playing" and no buttons. Reads `use_now_playing`; asks the service to re-read the
//! position when it opens, so the bar starts from where the track is now.

use std::time::Duration;

use dioxus::prelude::*;
use ds::{
    Availability, ExternalIcon, Icon, IconButton, IconButtonVariant, IconSize, IconSource, IconUrl,
    ModulePanel, NowPlayingTrack, PlayPauseButton, Playback, Text, TrackPosition,
};
use sill_services::{Art, MediaCommand, MediaVerb, Micros, PlayerId, Timeline};

use crate::hooks::{use_now_playing, use_services};
use crate::surfaces::control_center::modules::use_plate;
use crate::surfaces::control_center::track_lines;

/// The Now Playing module.
#[component]
pub fn NowPlayingModule() -> Element {
    let media = use_now_playing()();
    let control = use_services().media_control;
    let playing = media.current().cloned();
    let plate = use_plate();
    use_position_on_open(playing.as_ref().map(|player| player.id.clone()));
    let Some(player) = playing else {
        return rsx! {
            ModulePanel { plate,
                div { class: "sill-cc-media",
                    NowPlayingTrack { art: None::<IconSource>, title: "Not Playing" }
                }
            }
        };
    };
    let (title, by) = track_lines(&player);
    let press = move |verb: MediaVerb| {
        let control = control.clone();
        let player = player.id.clone();
        move |_| {
            // A stopped service drops it: the module shows what it last published.
            let _gone = control.send(MediaCommand::Press {
                player: player.clone(),
                verb,
            });
        }
    };
    let playback = playback_of(player.playback);
    let abilities = player.abilities;
    let by = (!by.is_empty()).then(|| Text::from(by));
    let position = position_of(player.timeline);
    rsx! {
        ModulePanel { plate,
            div { class: "sill-cc-media",
                div { class: "sill-cc-media-row",
                    div { class: "sill-cc-media-track",
                        NowPlayingTrack { art: art_of(&player.track.art), title, by, playback }
                    }
                    div { class: "sill-cc-media-buttons",
                        IconButton {
                            variant: IconButtonVariant::Tool,
                            icon: Icon::SkipBack,
                            label: "Previous".to_owned(),
                            availability: available(abilities.previous),
                            onclick: press(MediaVerb::Previous),
                        }
                        PlayPauseButton {
                            playback,
                            availability: available(abilities.play_pause),
                            onclick: press(MediaVerb::PlayPause),
                        }
                        IconButton {
                            variant: IconButtonVariant::Tool,
                            icon: Icon::SkipForward,
                            label: "Next".to_owned(),
                            availability: available(abilities.next),
                            onclick: press(MediaVerb::Next),
                        }
                    }
                }
                if let Some(TrackTimes { at, length }) = position {
                    TrackPosition { at, length, playback }
                }
            }
        }
    }
}

/// Asks the service once, when the module opens on a player, to re-read that player's position
/// (MPRIS sends no signal as a track plays, so the last report can be minutes old).
fn use_position_on_open(player: Option<PlayerId>) {
    let control = use_services().media_control;
    let mut asked = use_hook(|| CopyValue::new(None::<PlayerId>));
    if let Some(player) = player
        && asked.peek().as_ref() != Some(&player)
    {
        asked.set(Some(player.clone()));
        // A stopped service drops it: the bar holds the last report.
        let _gone = control.send(MediaCommand::Position(player));
    }
}

/// The service's playback as quire's: a stopped player offers Play, as a paused one does.
pub(crate) fn playback_of(playback: sill_services::Playback) -> Playback {
    match playback {
        sill_services::Playback::Playing => Playback::Playing,
        sill_services::Playback::Paused | sill_services::Playback::Stopped => Playback::Paused,
    }
}

/// Where the track is and how long it is, as `TrackPosition` takes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TrackTimes {
    pub at: Duration,
    pub length: Duration,
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

#[cfg(test)]
mod tests {
    use super::{TrackTimes, playback_of, position_of};
    use ds::Playback;
    use sill_services::{Micros, Stamp, Timeline};
    use std::time::Duration;

    #[test]
    fn a_stopped_player_offers_play() {
        let cases = [
            (sill_services::Playback::Playing, Playback::Playing),
            (sill_services::Playback::Paused, Playback::Paused),
            (sill_services::Playback::Stopped, Playback::Paused),
        ];
        for (from, want) in cases {
            assert_eq!(playback_of(from), want, "{from:?}");
        }
    }

    #[test]
    fn the_bar_shows_only_when_both_times_are_known() {
        let known = Timeline::Known {
            position: Micros(65_000_000),
            length: Micros(301_000_000),
            at: Stamp(9),
        };
        let cases = [
            (
                known,
                Some(TrackTimes {
                    at: Duration::from_secs(65),
                    length: Duration::from_secs(301),
                }),
            ),
            (Timeline::Unknown, None),
        ];
        for (timeline, want) in cases {
            assert_eq!(position_of(timeline), want, "{timeline:?}");
        }
    }
}
