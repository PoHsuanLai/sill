//! The widget contract (design/23-WIDGETS.md section 9): what a widget is, as one trait. A
//! widget is data in, content out: it names its kind and the sizes it draws, gives a placeholder
//! entry for a size, and draws an entry at a size. It never draws its card: the corner, the
//! inset, the material, the Space's tint and the title row are quire's ([`crate::widget::card::WidgetCard`]),
//! so every widget, ours and other apps', sits on the same card.

use crate::widget::kind::{WidgetHost, WidgetSize, WidgetTitle};
use dioxus::prelude::*;
use ds::motion::wake::WakeStamp;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// A widget's kind: a reverse-DNS name, unique across every app that provides widgets
/// (`quire.month`). What a card's markup names the widget by.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WidgetKind(Cow<'static, str>);

impl WidgetKind {
    /// A kind compiled into the program.
    pub const fn fixed(name: &'static str) -> Self {
        WidgetKind(Cow::Borrowed(name))
    }

    /// A kind read at run time (a settings file, another app's announcement).
    pub fn named(name: impl Into<String>) -> Self {
        WidgetKind(Cow::Owned(name.into()))
    }

    /// The name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An intent for a widget that has no controls: it has no values, so a passive widget's
/// `onintent` can never be called.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NoIntent {}

/// What an entry is drawn with: the size it was given (always one of the widget's own
/// [`Widget::sizes`]), the host, the host's wake stamp (a widget replays its appear motion on a
/// new one: the battery's fill), and the handler for the widget's own controls.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetContext<I: 'static> {
    /// The card's size.
    pub size: WidgetSize,
    /// The desktop or the notification center.
    pub host: WidgetHost,
    /// The host's wake stamp: new when its widgets come into view.
    pub wake: WakeStamp,
    /// Where the widget's controls send their intents; `None` draws the controls inert.
    pub act: Option<EventHandler<I>>,
}

/// A widget: one implementation per kind, a unit type (`Default`). The shell's are
/// [`crate::widget::calendar::MonthWidget`] and sill's Up Next.
///
/// `Entry` is everything one moment of the widget shows, as plain data. `view` is a pure function of the entry
/// and the context: it returns markup (it may mount components, which may keep their own motion
/// state) and calls no hook itself.
pub trait Widget: Clone + PartialEq + Default + 'static {
    /// One moment of the widget.
    type Entry: Clone + PartialEq + Serialize + DeserializeOwned + 'static;
    /// What the widget's controls ask of its provider; [`NoIntent`] for a widget without any.
    type Intent: Clone + PartialEq + Serialize + DeserializeOwned + 'static;

    /// The kind.
    fn kind() -> WidgetKind;

    /// The sizes it draws, the first the one it is added at.
    fn sizes() -> &'static [WidgetSize];

    /// What it shows before its provider's first entry: honest about having no data (bare
    /// tracks, not a made-up level).
    fn placeholder(size: WidgetSize) -> Self::Entry;

    /// The content of `entry` in `cx`, without the card.
    fn view(entry: &Self::Entry, cx: WidgetContext<Self::Intent>) -> Element;

    /// The card's title row; none by default (a glanceable widget's content fills the card,
    /// design/23 section 2 rule 4).
    fn title() -> Option<WidgetTitle> {
        None
    }
}

/// `size` if `W` draws it, else the first size `W` does: a card never asks a widget for a size
/// it has not drawn.
pub fn fit<W: Widget>(size: WidgetSize) -> WidgetSize {
    let sizes = W::sizes();
    if sizes.contains(&size) {
        size
    } else {
        sizes.first().copied().unwrap_or_default()
    }
}
