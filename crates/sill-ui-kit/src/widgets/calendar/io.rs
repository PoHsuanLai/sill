//! The Calendar widget's view: reads `use_service::<EventsService>` (today, the events) and the
//! `calendar` settings, keeps the month shown in a signal (today's month on mount, stepped by the
//! card's `MonthIntent::Step`, back to today's month when the day turns), and hands quire's
//! `MonthWidget` one entry: the grid from [`super::grid::month_grid`] and
//! [`super::grid::busy_days`], today's weekday and date, and today's events from the agenda Up Next
//! reads ([`crate::widgets::up_next::today_events`]). Redrawn on the clock's minute tick.

use dioxus::prelude::*;
use ds::{MonthEntry, MonthFace, MonthIntent, TodayLine};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use sill_settings::{CalendarSettings, WeekNumbers};

use super::grid::{Step, busy_days, month_grid};
use super::shown::{Shown, grid_data, weekday_name};
use crate::hooks::{use_section, use_service};
use crate::widgets::up_next::today_events;
use crate::widgets::{Feed, WidgetKind, WidgetProvider, WidgetSize};
use sill_services::{ClockService, EventsService};

/// The month, today marked; the card's step buttons shift the month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CalendarProvider;

impl WidgetProvider for CalendarProvider {
    type W = ds::MonthWidget;
    const KIND: WidgetKind = WidgetKind::Calendar;
    const DESKTOP_SIZE: WidgetSize = WidgetSize::Small;

    fn use_feed() -> Feed<ds::MonthWidget> {
        let _minute = use_service::<ClockService>()();
        let agenda = use_service::<EventsService>()();
        let calendar = use_section::<CalendarSettings>()();
        let today = agenda.today;
        let mut shown = use_signal(|| Shown::today(today));
        let month = shown().month(today);
        let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
        let busy = busy_days(&agenda.events, &zone, month);
        let grid = month_grid(month, calendar.first_weekday, today, &busy);
        let weeks = match calendar.week_numbers {
            WeekNumbers::Hide => ds::WeekNumbers::Hide,
            WeekNumbers::Show => ds::WeekNumbers::Show,
        };
        let face = MonthFace {
            today: Some(TodayLine {
                weekday: weekday_name(today).to_owned(),
                day: u8::try_from(today.day()).unwrap_or(1),
            }),
            events: today_events(&agenda, Timestamp::now(), &zone),
            no_events: "No events today".to_owned(),
            ..MonthFace::of(grid_data(&grid, today), weeks)
        };
        let onintent = EventHandler::new(move |intent: MonthIntent| {
            let MonthIntent::Step(step) = intent;
            let step = match step {
                ds::MonthStep::Previous => Step::Previous,
                ds::MonthStep::Next => Step::Next,
            };
            shown.set(shown().stepped(today, step));
        });
        Feed {
            entry: MonthEntry::Month(Box::new(face)),
            onintent: Some(onintent),
        }
    }
}
