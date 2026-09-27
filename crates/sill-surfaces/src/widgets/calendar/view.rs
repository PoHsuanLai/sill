//! The Calendar widget's view: reads `use_agenda` (today, the events) and the `calendar`
//! settings, keeps the month shown in a signal (today's month on mount, stepped by the card's
//! `MonthIntent::Step`, back to today's month when the day turns), and hands quire's
//! `MonthWidget` one entry: the grid from [`super::grid::month_grid`] and
//! [`super::grid::busy_days`], today's weekday and date, and today's events from the agenda Up
//! Next reads ([`crate::widgets::up_next::today_events`]). Redrawn on the clock's minute tick.

use dioxus::prelude::*;
use ds::{MonthEntry, MonthFace, MonthIntent, TodayLine, WidgetCard};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use sill_settings::WeekNumbers;

use super::grid::{Step, busy_days, month_grid};
use super::shown::{Shown, grid_data, weekday_name};
use crate::hooks::{use_agenda, use_clock, use_settings};
use crate::widgets::up_next::today_events;
use crate::widgets::{Place, WidgetKind, use_widget_wake, widget_element_id};

/// The month, today marked, on quire's card.
#[component]
pub fn CalendarCard(place: Place) -> Element {
    let _minute = use_clock()();
    let agenda = use_agenda()();
    let calendar = use_settings()().calendar;
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
            ds::Step::Previous => Step::Previous,
            ds::Step::Next => Step::Next,
        };
        shown.set(shown().stepped(today, step));
    });
    rsx! {
        WidgetCard {
            widget: ds::MonthWidget,
            timeline: ds::Timeline::now(MonthEntry::Month(Box::new(face))),
            size: place.size,
            host: place.host,
            wake: use_widget_wake(),
            id: widget_element_id(WidgetKind::Calendar),
            onintent,
            lift: place.lift,
        }
    }
}
