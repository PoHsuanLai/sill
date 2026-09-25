//! The Calendar widget's view: reads `use_agenda` (today, the events) and the `calendar`
//! settings, keeps the month shown in a signal (today's month on mount, stepped by the previous
//! and next buttons, back to today's month when the day turns), and draws quire's `MonthGrid`
//! (Q180) from [`super::grid::month_grid`] and [`super::grid::busy_days`]: macOS's calendar
//! widget, the month's name in the accent, weekday initials, today on the accent disc, a dot
//! under each day with an event. On the desktop, whose small cell quire's month does not fit
//! (Q190), macOS's small date face instead. Redrawn on the clock's minute tick.

use dioxus::prelude::*;
use jiff::tz::TimeZone;
use sill_settings::WeekNumbers;

use super::grid::{Month, Step, busy_days, month_grid};
use super::shown::{Shown, grid_data, month_title, weekday_name};
use crate::hooks::{use_agenda, use_clock, use_settings};
use crate::widgets::WidgetHost;

/// The month, today marked.
#[component]
pub fn CalendarWidget(host: WidgetHost) -> Element {
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
    let onstep = EventHandler::new(move |step: ds::Step| {
        let step = match step {
            ds::Step::Previous => Step::Previous,
            ds::Step::Next => Step::Next,
        };
        shown.set(shown().stepped(today, step));
    });
    match host {
        WidgetHost::Tile => rsx! {
            div { class: "sill-cal sill-cal-center",
                ds::MonthGrid { data: grid_data(&grid, today), weeks, onstep }
            }
        },
        // quire's month does not fit a small desktop cell (Q190): macOS's small date face.
        WidgetHost::Desktop => rsx! {
            div { class: "sill-cal sill-cal-desktop",
                span { class: "sill-cal-weekday", {weekday_name(today.weekday())} }
                span { class: "sill-cal-date", {today.day().to_string()} }
                span { class: "sill-cal-month", {month_title(Month::of(today), today)} }
            }
        },
    }
}
