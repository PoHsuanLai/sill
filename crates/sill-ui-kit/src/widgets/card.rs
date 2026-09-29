//! One widget of any kind: the match on the kind that picks its provider's card.

use dioxus::prelude::*;
use ds::{CardPresence, Lift, WidgetHost};
use sill_settings::WidgetKind;

use super::{Place, battery, calendar, now_playing, size_of, up_next, world_clock};

/// One widget of `kind` in `host` at its one size ([`size_of`]), on quire's card, named by [`widget_element_id`],
/// `lift`ed while the desktop drags it: the kind's provider feeds the card. A card the desktop
/// took away is `Leaving`: it plays quire's exit and calls `on_gone` once it has.
#[component]
pub fn Widget(
    kind: WidgetKind,
    host: WidgetHost,
    #[props(default)] lift: Lift,
    #[props(default)] presence: CardPresence,
    #[props(default)] on_gone: Option<EventHandler<()>>,
) -> Element {
    let place = Place {
        host,
        size: size_of(kind, host),
        lift,
        presence,
        on_gone,
    };
    match kind {
        WidgetKind::Calendar => rsx! { calendar::CalendarCard { place } },
        WidgetKind::UpNext => rsx! { up_next::UpNextCard { place } },
        WidgetKind::Battery => rsx! { battery::BatteryCard { place } },
        WidgetKind::NowPlaying => rsx! { now_playing::NowPlayingCard { place } },
        WidgetKind::WorldClock => rsx! { world_clock::WorldClockCard { place } },
    }
}
