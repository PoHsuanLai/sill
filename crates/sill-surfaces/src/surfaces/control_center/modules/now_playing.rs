//! The Now Playing module: `Media::current`'s track on the module's panel, drawn by the view it
//! shares with the Now Playing widget ([`super::media_face::MediaView`]); a press sends
//! `MediaCommand::Press`. Reads `use_service::<NowPlayingService>`; asks the service to re-read the
//! position when it opens, so the bar starts from where the track is now.

use dioxus::prelude::*;
use ds::ModulePanel;
use sill_services::{MediaCommand, NowPlayingService, PlayerId};

use super::media_face::{MediaView, Transport, media_face};
use crate::hooks::{use_control, use_service};
use crate::surfaces::control_center::modules::use_plate;

/// The Now Playing module.
#[component]
pub fn NowPlayingModule() -> Element {
    let plate = use_plate();
    let (face, onpress) = use_media();
    rsx! {
        ModulePanel { plate,
            MediaView { face, onpress }
        }
    }
}

/// The current player's face and where its presses go: read by the module and the widget.
pub(crate) fn use_media() -> (super::media_face::MediaFace, EventHandler<Transport>) {
    let media = use_service::<NowPlayingService>()();
    let control = use_control::<NowPlayingService>();
    let playing = media.current().cloned();
    use_position_on_open(playing.as_ref().map(|player| player.id.clone()));
    let face = media_face(playing.as_ref());
    let player = playing.map(|player| player.id);
    let onpress = EventHandler::new(move |transport: Transport| {
        if let Some(player) = player.clone() {
            // A stopped service drops it: the view shows what it last published.
            let _gone = control.send(MediaCommand::Press {
                player,
                verb: transport.verb(),
            });
        }
    });
    (face, onpress)
}

/// Asks the service once, when the module opens on a player, to re-read that player's position
/// (MPRIS sends no signal as a track plays, so the last report can be minutes old).
fn use_position_on_open(player: Option<PlayerId>) {
    let control = use_control::<NowPlayingService>();
    let mut asked = use_hook(|| CopyValue::new(None::<PlayerId>));
    if let Some(player) = player
        && asked.peek().as_ref() != Some(&player)
    {
        asked.set(Some(player.clone()));
        // A stopped service drops it: the bar holds the last report.
        let _gone = control.send(MediaCommand::Position(player));
    }
}

#[cfg(test)]
mod tests {
    use super::super::media_face::{TrackTimes, playback_of, position_of};
    use ds::Playback;
    use sill_model::units::Stamp;
    use sill_services::{Micros, Timeline};
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
