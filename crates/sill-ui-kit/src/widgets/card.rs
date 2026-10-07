//! One widget of any kind: the [`WidgetProvider`] every kind implements, the [`Card`] that puts a
//! provider's feed on quire's card, and the one match on the kind ([`with_provider`]).

use dioxus::prelude::*;
use ds::prelude::*;
use ds::root::common::Common;
use sill_settings::WidgetKind;
use sill_shell_kit::prelude::*;
use sill_shell_kit::widget::kind::Lift;
use sill_shell_kit::widget::timeline::Timeline;

use super::{
    Place, battery::BatteryProvider, calendar::CalendarProvider, now_playing::NowPlayingProvider,
    up_next::UpNextProvider, use_widget_wake, widget_element_id, world_clock::WorldClockProvider,
};

/// What a provider hands the card: the entry now, and where the widget's controls go (none for
/// a widget without any).
pub struct Feed<W: sill_shell_kit::prelude::Widget> {
    pub entry: W::Entry,
    pub onintent: Option<EventHandler<W::Intent>>,
}

impl<W: sill_shell_kit::prelude::Widget> std::fmt::Debug for Feed<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Feed")
            .field("intents", &self.onintent.is_some())
            .finish_non_exhaustive()
    }
}

/// One kind's provider: the `sill_shell_kit::prelude::Widget` it feeds, its settings kind, and the hook that reads
/// its services into a [`Feed`]. The `sill_shell_kit::prelude::Widget` is a value ([`Default`]); its size on the
/// desktop is [`Self::DESKTOP_SIZE`](WidgetProvider::DESKTOP_SIZE) (the notification center's
/// tiles are all Medium, see [`size_of`](super::size_of)); its name in quire's registry is
/// [`registry_id`](WidgetProvider::registry_id).
pub trait WidgetProvider: Clone + PartialEq + Default + 'static {
    type W: sill_shell_kit::prelude::Widget;
    const KIND: WidgetKind;
    const DESKTOP_SIZE: WidgetSize;
    fn use_feed() -> Feed<Self::W>;

    /// The widget's name in quire's registry.
    fn registry_id() -> sill_shell_kit::prelude::WidgetKind {
        <Self::W as sill_shell_kit::prelude::Widget>::kind()
    }
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
            common: Common { id: Some(widget_element_id(P::KIND)), ..Common::default() },
            onintent,
            lift: place.lift,
            shown: place.shown,
            on_hidden: place.on_hidden,
        }
    }
}

/// What to do with one kind's provider type: [`with_provider`] picks the type from the kind.
pub trait OnProvider {
    type Out;
    fn on<P: WidgetProvider>(self) -> Self::Out;
}

/// `action` on `kind`'s provider: the only match on the kind.
pub fn with_provider<A: OnProvider>(kind: WidgetKind, action: A) -> A::Out {
    match kind {
        WidgetKind::Calendar => action.on::<CalendarProvider>(),
        WidgetKind::UpNext => action.on::<UpNextProvider>(),
        WidgetKind::Battery => action.on::<BatteryProvider>(),
        WidgetKind::NowPlaying => action.on::<NowPlayingProvider>(),
        WidgetKind::WorldClock => action.on::<WorldClockProvider>(),
    }
}

struct CardAt(Place);

impl OnProvider for CardAt {
    type Out = Element;
    fn on<P: WidgetProvider>(self) -> Element {
        rsx! { Card { provider: P::default(), place: self.0 } }
    }
}

/// `kind`'s card at `place`.
pub fn card(kind: WidgetKind, place: Place) -> Element {
    with_provider(kind, CardAt(place))
}

/// One widget of `kind` in `host` at its one size ([`size_of`]), on quire's card, `lift`ed while
/// the desktop drags it: the kind's provider feeds the card. A card the desktop took away is
/// `Hidden`: it plays quire's exit and calls `on_hidden` once it has.
#[component]
pub fn Widget(
    kind: WidgetKind,
    host: WidgetHost,
    #[props(default)] lift: Lift,
    #[props(default = Shown::Visible)] shown: Shown,
    #[props(default)] on_hidden: Option<EventHandler<()>>,
) -> Element {
    let place = Place {
        host,
        size: super::size_of(kind, host),
        lift,
        shown,
        on_hidden,
    };
    card(kind, place)
}
