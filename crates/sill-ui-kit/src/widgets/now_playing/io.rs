//! The Now Playing widget on quire's card, and the hook that reads the current player: the
//! control center's module reads the same one.

use dioxus::prelude::*;
use sill_model::now_playing::{MediaCommand, PlayerId};
use sill_services::NowPlayingService;

use super::face::{MediaFace, Transport, media_face};
use super::widget::NowPlaying;
use crate::hooks::{use_control, use_service};
use crate::widgets::{Feed, WidgetKind, WidgetProvider};

/// The current player on quire's card; a press goes to that player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NowPlayingProvider;

impl WidgetProvider for NowPlayingProvider {
    type W = NowPlaying;
    const KIND: WidgetKind = WidgetKind::NowPlaying;

    fn use_feed() -> Feed<NowPlaying> {
        let (entry, onintent) = use_media();
        Feed {
            entry,
            onintent: Some(onintent),
        }
    }
}

/// The current player's face and where its presses go: read by the module and the widget.
pub fn use_media() -> (MediaFace, EventHandler<Transport>) {
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
