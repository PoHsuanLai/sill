//! Up Next's feed, from the services.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use sill_model::events::Days;
use sill_services::{ClockService, EventsService};

use super::provider::{EVENT_ROWS, up_next_entry};
use super::widget::UpNext;
use crate::hooks::{use_hours, use_service};
use crate::widgets::{Feed, WidgetKind, WidgetProvider};

/// The next events, as many as the card's body holds whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UpNextProvider;

/// The most events asked for; the card shows as many as it has room for.
const UP_NEXT_MAX: usize = 3;
/// How many days ahead the events are taken from.
const UP_NEXT_DAYS: u16 = 7;

impl WidgetProvider for UpNextProvider {
    type W = UpNext;
    const KIND: WidgetKind = WidgetKind::UpNext;

    fn use_feed() -> Feed<UpNext> {
        let _minute = use_service::<ClockService>()();
        let hours = use_hours();
        let agenda = use_service::<EventsService>()();
        let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
        let max = UP_NEXT_MAX.min(EVENT_ROWS);
        let days = Days(UP_NEXT_DAYS);
        Feed {
            entry: up_next_entry(&agenda, Timestamp::now(), &zone, days, max, hours),
            onintent: None,
        }
    }
}
