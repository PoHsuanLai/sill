//! The Now Playing widget (kind `sill.now-playing`): the control center's Now Playing reused, not
//! redrawn. Its entry is the control center's face as data
//! (`surfaces::control_center::modules::media_face::MediaFace`) and its view the same
//! `MediaView`; its intents are the transport's presses, which the provider sends to the
//! current player. The module's layout rules live in the control center's stylesheet, which the
//! widgets' hosts do not load, so `widget_now_playing.css` carries the same few rules for its
//! classes.

use dioxus::prelude::*;
use ds::Word;
use ds::{Icon, TextLine, WidgetCard, WidgetContext, WidgetSize, WidgetTitle};
use sill_model::units::Stamp;
use sill_services::{Abilities, Art, Availability, Micros, Playback, Timeline};

use super::{Place, WidgetKind, use_widget_wake, widget_element_id};
use crate::surfaces::control_center::modules::media_face::{
    MediaFace, MediaView, TrackFace, Transport, position_of,
};
use crate::surfaces::control_center::modules::now_playing::use_media;

/// This widget's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("../style/widget_now_playing.css");

/// The Now Playing widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NowPlaying;

impl ds::Widget for NowPlaying {
    type Entry = MediaFace;
    type Intent = Transport;

    fn kind() -> ds::WidgetKind {
        ds::WidgetKind::fixed("sill.now-playing")
    }

    fn name() -> TextLine {
        TextLine::from("Now Playing")
    }

    fn description() -> TextLine {
        TextLine::from("See what is playing and control it.")
    }

    fn sizes() -> &'static [WidgetSize] {
        &[WidgetSize::Medium]
    }

    fn placeholder(_size: WidgetSize) -> MediaFace {
        MediaFace::Idle
    }

    fn preview(_size: WidgetSize) -> MediaFace {
        let enabled = Availability::Enabled;
        MediaFace::Track(TrackFace {
            title: "Clair de lune".to_owned(),
            by: "Claude Debussy — Suite bergamasque".to_owned(),
            art: Art::None,
            playback: Playback::Playing,
            abilities: Abilities {
                previous: enabled,
                play_pause: enabled,
                next: enabled,
            },
            times: position_of(Timeline::Known {
                position: Micros(95_000_000),
                length: Micros(301_000_000),
                at: Stamp(0),
            }),
        })
    }

    fn view(entry: &MediaFace, cx: WidgetContext<Transport>) -> Element {
        rsx! {
            div { class: "sill-widget-now-playing", "data-host": cx.host.slug(),
                MediaView { face: entry.clone(), onpress: cx.act }
            }
        }
    }

    fn title() -> Option<WidgetTitle> {
        Some(WidgetTitle::new(Icon::Play, "Now Playing"))
    }
}

/// The current player on quire's card; a press goes to that player.
#[component]
pub fn NowPlayingCard(place: Place) -> Element {
    let (face, onintent) = use_media();
    rsx! {
        WidgetCard {
            widget: NowPlaying,
            timeline: ds::Timeline::now(face),
            size: place.size,
            host: place.host,
            wake: use_widget_wake(),
            id: widget_element_id(WidgetKind::NowPlaying),
            onintent,
            lift: place.lift,
            presence: place.presence,
            on_gone: place.on_gone,
        }
    }
}
