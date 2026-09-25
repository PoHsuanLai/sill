//! One notification drawn as a card (design/13 §13.3.6 "Banner", "Content"): the app icon (32)
//! at the left, the app name and the age on the top line, the summary (ui 13/700) and the body
//! (ui 13, two lines, six when expanded) with its runs styled, the count chip for a group, and,
//! when expanded, the action buttons (`Button{Mini}`) and the close button at the top-left
//! corner. Shared by the banners and the notification center, so both read alike.
//!
//! quire has no notification card yet (FINDINGS "M6 freeze", Q120); until it does, lane b builds
//! this from quire's parts only (glyphs, `Count`, `Button{Mini}`, `IconButton`), never a local
//! component that paints its own material.

pub mod actions;
pub mod body;
pub mod icon;
pub mod labels;
pub mod metrics;

mod view;

pub use actions::{MenuOpen, MenuSeat};
pub use labels::{Age, age, age_label, app_label, count_label};
pub use metrics::{CARD_VARS, CardMetrics};
pub use view::{CARD_CSS, wall_now};

use dioxus::prelude::*;
use sill_services::{ActionKey, Href, Notification, UnixMs};

/// Whether the card shows its full body and its buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Expansion {
    #[default]
    Collapsed,
    /// Hovered (a banner) or opened (a center row).
    Expanded,
}

/// Where the card is drawn: a banner paints the Toast material; a center row sits on the
/// center's Popover panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardPlace {
    Banner,
    Center,
}

/// What was pressed on a card.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CardPress {
    /// The body: the `default` action when the notification offers one.
    Body,
    /// One of its action buttons.
    Action(ActionKey),
    /// The close button.
    Close,
    /// A link in the body.
    Link(Href),
}

/// `note` as a card; `count` notifications stand behind it (1: no chip).
#[component]
pub fn NotificationCard(
    note: Notification,
    count: usize,
    expansion: Expansion,
    place: CardPlace,
    now: UnixMs,
    on_press: EventHandler<CardPress>,
) -> Element {
    view::card(note, count, expansion, place, now, on_press)
}
