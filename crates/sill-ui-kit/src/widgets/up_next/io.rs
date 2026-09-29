//! Up Next on quire's card, fed from the services.

use dioxus::prelude::*;
use ds::WidgetCard;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use sill_model::events::Days;
use sill_services::{ClockService, EventsService};

use super::provider::{EVENT_ROWS, up_next_entry};
use super::widget::UpNext;
use crate::hooks::{use_service, use_settings};
use crate::widgets::{Place, WidgetKind, use_widget_wake, widget_element_id};

/// The next events on quire's card, as many as its body holds whole.
#[component]
pub fn UpNextCard(place: Place) -> Element {
    let _minute = use_service::<ClockService>()();
    let agenda = use_service::<EventsService>()();
    let calendar = use_settings()().calendar;
    let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
    let max = usize::from(calendar.up_next_max.0).min(EVENT_ROWS);
    let days = Days(calendar.up_next_days.0);
    let entry = up_next_entry(&agenda, Timestamp::now(), &zone, days, max);
    rsx! {
        WidgetCard {
            widget: UpNext,
            timeline: ds::Timeline::now(entry),
            size: place.size,
            host: place.host,
            wake: use_widget_wake(),
            id: widget_element_id(WidgetKind::UpNext),
            lift: place.lift,
            presence: place.presence,
            on_gone: place.on_gone,
        }
    }
}
