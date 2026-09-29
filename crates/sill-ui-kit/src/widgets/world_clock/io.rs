//! The world clocks on quire's card, fed from the clock service.

use dioxus::prelude::*;
use ds::WidgetCard;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use sill_services::ClockService;

use super::provider::{clock_entry, clock_rows, parse_zones};
use crate::hooks::{use_service, use_settings};
use crate::widgets::{Place, WidgetKind, use_widget_wake, widget_element_id};

/// The clocks on quire's card. The clock service's minute tick is the only thing that makes a
/// new entry: no timer of its own.
#[component]
pub fn WorldClockCard(place: Place) -> Element {
    let _minute = use_service::<ClockService>()();
    let zones = parse_zones(&use_settings()().widgets.world_clocks);
    let entry = clock_entry(clock_rows(Timestamp::now(), &zones, &TimeZone::system()));
    rsx! {
        WidgetCard {
            widget: ds::WorldClockWidget,
            timeline: ds::Timeline::now(entry),
            size: place.size,
            host: place.host,
            wake: use_widget_wake(),
            id: widget_element_id(WidgetKind::WorldClock),
            lift: place.lift,
            presence: place.presence,
            on_gone: place.on_gone,
        }
    }
}
