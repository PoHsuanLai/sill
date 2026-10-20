//! Up Next's feed, from the services.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use sill_model::events::Days;
use sill_services::{ClockService, EventsService};

use super::provider::{EVENT_ROWS, up_next_entry};
use super::widget::UpNext;
use crate::hooks::{use_hours, use_section, use_service};
use crate::widgets::{Feed, WidgetKind, WidgetProvider};

/// The next events, as many as the card's body holds whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UpNextProvider;

impl WidgetProvider for UpNextProvider {
    type W = UpNext;
    const KIND: WidgetKind = WidgetKind::UpNext;

    fn use_feed() -> Feed<UpNext> {
        let _minute = use_service::<ClockService>()();
        let hours = use_hours();
        let agenda = use_service::<EventsService>()();
        let calendar = use_section(|file| &file.calendar)();
        let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
        let max = usize::from(calendar.up_next_max.0).min(EVENT_ROWS);
        let days = Days(calendar.up_next_days.0);
        Feed {
            entry: up_next_entry(&agenda, Timestamp::now(), &zone, days, max, hours),
            onintent: None,
        }
    }
}
