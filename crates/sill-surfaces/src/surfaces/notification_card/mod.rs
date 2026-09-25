//! One notification drawn as quire's `NotificationCard` (design/13 §13.3.6 "Banner", "Content";
//! quire CONSUMING.md "Notification parts"), shared by the banners and the notification center
//! so both read alike. sill supplies the words (the app's icon and name, the age, the body's
//! runs, the actions) and hears the presses; quire draws the plate, the two-to-six-line body,
//! the actions and the close button on hover, the group's count and layers, and the swipe.

pub mod actions;
pub mod body;
pub mod icon;
pub mod labels;
pub mod metrics;
pub mod parts;

mod view;

pub use body::rich_body;
pub use labels::{Age, age, age_label, app_label, group_count};
pub use metrics::{icon_size, notification_metrics, swipe_metrics};
pub use parts::{CardPart, part_id};
pub use view::{use_app_mark, wall_now};

use dioxus::prelude::*;
use ds::{Hover, Swipe};
use sill_services::{ActionKey, Href, Notification, UnixMs};

/// Where the card is drawn: a banner paints the Toast material (ink on paper under
/// `notifications.banner_material = "inverse"`); a center row the center's Popover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardPlace {
    Banner,
    Center,
}

/// What was pressed on a card.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CardPress {
    /// The card itself (a press, Enter or Space): the `default` action when it offers one.
    Body,
    /// One of its action buttons.
    Action(ActionKey),
    /// The close button.
    Close,
    /// A link in the body.
    Link(Href),
}

/// `note` as a card; `count` notifications stand behind it (1: no chip, no layers). `id` names
/// quire's plate for a region (a banner's `toast-<id>`); `swipe` lets a drag dismiss it;
/// `on_hover` hears the pointer come and go.
#[component]
pub fn NoteCard(
    note: Notification,
    count: usize,
    place: CardPlace,
    now: UnixMs,
    #[props(default)] id: Option<String>,
    #[props(default)] swipe: Swipe,
    #[props(default)] on_hover: Option<EventHandler<Hover>>,
    on_press: EventHandler<CardPress>,
) -> Element {
    view::card(
        note,
        count,
        place,
        now,
        view::CardWiring {
            id,
            swipe,
            on_hover,
            on_press,
        },
    )
}
