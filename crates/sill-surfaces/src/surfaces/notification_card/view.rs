//! The card's markup (lane b, FINDINGS "The M6 fill-in"): quire parts only.
//!
//! Frozen stub: draws nothing yet.

use dioxus::prelude::*;
use sill_services::{Notification, UnixMs};

use super::{CardPlace, CardPress, Expansion};

/// `note` as a card.
pub(super) fn card(
    note: Notification,
    count: usize,
    expansion: Expansion,
    place: CardPlace,
    now: UnixMs,
    on_press: EventHandler<CardPress>,
) -> Element {
    let _ = (note, count, expansion, place, now, on_press);
    rsx! {}
}
