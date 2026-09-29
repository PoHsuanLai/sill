//! One widget of any kind: the [`WidgetProvider`] every kind implements, the [`Card`] that puts a
//! provider's feed on quire's card, and the one match on the kind ([`card`]).

use dioxus::prelude::*;
use ds::{CardPresence, Lift, Timeline, WidgetCard, WidgetHost};
use sill_settings::WidgetKind;

use super::{
    Place, battery::BatteryProvider, calendar::CalendarProvider, now_playing::NowPlayingProvider,
    size_of, up_next::UpNextProvider, use_widget_wake, widget_element_id,
    world_clock::WorldClockProvider,
};

/// What a provider hands the card: the entry now, and where the widget's controls go (none for
/// a widget without any).
pub struct Feed<W: ds::Widget> {
    pub entry: W::Entry,
    pub onintent: Option<EventHandler<W::Intent>>,
}

/// One kind's provider: the `ds::Widget` it feeds, its settings kind, and the hook that reads
/// its services into a [`Feed`]. The `ds::Widget` is a value ([`Default`]); its size is
/// [`size_of`].
pub trait WidgetProvider: Clone + PartialEq + 'static {
    type W: ds::Widget;
    const KIND: WidgetKind;
    fn use_feed() -> Feed<Self::W>;
}

/// `provider`'s widget on quire's card at `place`, named by [`widget_element_id`]; the provider
/// value only names the type.
#[component]
pub fn Card<P: WidgetProvider>(provider: P, place: Place) -> Element {
    let _ = provider;
    let Feed { entry, onintent } = P::use_feed();
    rsx! {
        WidgetCard {
            widget: <P::W as Default>::default(),
            timeline: Timeline::now(entry),
            size: place.size,
            host: place.host,
            wake: use_widget_wake(),
            id: widget_element_id(P::KIND),
            onintent,
            lift: place.lift,
            presence: place.presence,
            on_gone: place.on_gone,
        }
    }
}

/// `kind`'s card at `place`: the only match on the kind.
pub fn card(kind: WidgetKind, place: Place) -> Element {
    match kind {
        WidgetKind::Calendar => rsx! { Card { provider: CalendarProvider, place } },
        WidgetKind::UpNext => rsx! { Card { provider: UpNextProvider, place } },
        WidgetKind::Battery => rsx! { Card { provider: BatteryProvider, place } },
        WidgetKind::NowPlaying => rsx! { Card { provider: NowPlayingProvider, place } },
        WidgetKind::WorldClock => rsx! { Card { provider: WorldClockProvider, place } },
    }
}

/// One widget of `kind` in `host` at its one size ([`size_of`]), on quire's card, `lift`ed while
/// the desktop drags it: the kind's provider feeds the card. A card the desktop took away is
/// `Leaving`: it plays quire's exit and calls `on_gone` once it has.
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
    card(kind, place)
}
