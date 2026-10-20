//! One widget of any kind: the [`WidgetProvider`] every kind implements, the [`Card`] that puts a
//! provider's feed on quire's card, and the one match on the kind ([`with_provider`]).

use dioxus::prelude::*;
use ds::prelude::*;
use ds::root::common::Common;
use sill_settings::WidgetKind;
use sill_shell_kit::prelude::*;
use sill_shell_kit::widget::timeline::Timeline;

use super::{SIZE, calendar::CalendarProvider, up_next::UpNextProvider, use_widget_wake};

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

/// One kind's provider: the `sill_shell_kit::prelude::Widget` it feeds, its kind, and the hook
/// that reads its services into a [`Feed`].
pub trait WidgetProvider: Clone + PartialEq + Default + 'static {
    type W: sill_shell_kit::prelude::Widget;
    const KIND: WidgetKind;
    fn use_feed() -> Feed<Self::W>;
}

/// `provider`'s widget on quire's card in the notification center, its element id
/// `widget-<kind>`; the provider value only names the type.
#[component]
pub fn Card<P: WidgetProvider>(provider: P) -> Element {
    let _ = provider;
    let Feed { entry, onintent } = P::use_feed();
    rsx! {
        WidgetCard {
            widget: <P::W as Default>::default(),
            timeline: Timeline::now(entry),
            size: SIZE,
            wake: use_widget_wake(),
            common: Common { id: Some(format!("widget-{}", P::KIND.slug())), ..Common::default() },
            onintent,
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
    }
}

struct CardOf;

impl OnProvider for CardOf {
    type Out = Element;
    fn on<P: WidgetProvider>(self) -> Element {
        rsx! { Card { provider: P::default() } }
    }
}

/// `kind`'s card.
pub fn card(kind: WidgetKind) -> Element {
    with_provider(kind, CardOf)
}

/// One widget of `kind` on quire's card in the notification center: the kind's provider feeds
/// the card.
#[component]
pub fn Widget(kind: WidgetKind) -> Element {
    card(kind)
}
