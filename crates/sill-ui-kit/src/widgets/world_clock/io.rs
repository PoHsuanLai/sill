//! The world clocks' feed, from the clock service.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use sill_services::ClockService;
use sill_shell_kit::widget::clock::WorldClockWidget;

use super::provider::{clock_entry, clock_rows, parse_zones};
use crate::hooks::{use_service, use_settings};
use crate::widgets::{Feed, WidgetKind, WidgetProvider, WidgetSize};

/// The clocks. The clock service's minute tick is the only thing that makes a new entry: no
/// timer of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorldClockProvider;

impl WidgetProvider for WorldClockProvider {
    type W = WorldClockWidget;
    const KIND: WidgetKind = WidgetKind::WorldClock;
    const DESKTOP_SIZE: WidgetSize = WidgetSize::Medium;

    fn use_feed() -> Feed<WorldClockWidget> {
        let _minute = use_service::<ClockService>()();
        let zones = parse_zones(&use_settings()().widgets.world_clocks);
        Feed {
            entry: clock_entry(clock_rows(Timestamp::now(), &zones, &TimeZone::system())),
            onintent: None,
        }
    }
}
