//! The Now Playing widget (kind `sill.now-playing`): the control center's Now Playing reused, not
//! redrawn. Its entry is the control center's face as data
//! (`face::MediaFace`) and its view the same
//! `MediaView`; its intents are the transport's presses, which the provider sends to the
//! current player. The module's layout rules live in the control center's stylesheet, which the
//! widgets' hosts do not load, so `style.css` carries the same few rules for its
//! classes.

use dioxus::prelude::*;
use ds::base::time::stamp::Stamp;
use ds::prelude::*;
use ds_shell::prelude::*;
use ds_shell::widget::kind::WidgetTitle;
use sill_model::now_playing::{Abilities, Art, Micros, Playback, Timeline};
use sill_model::units::Availability;

use super::face::{MediaFace, MediaView, TrackFace, Transport, position_of};

/// The Now Playing widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NowPlaying;

impl Widget for NowPlaying {
    type Entry = MediaFace;
    type Intent = Transport;

    fn kind() -> WidgetKind {
        WidgetKind::fixed("sill.now-playing")
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
