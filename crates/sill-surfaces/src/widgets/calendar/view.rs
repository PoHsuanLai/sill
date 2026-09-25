//! The Calendar widget's view: reads `use_agenda` (today, the events) and the `calendar`
//! settings, keeps the month shown in a signal (today's month on mount, stepped by the previous
//! and next buttons, back to today's month when the day turns), and draws quire's `MonthGrid`
//! (Q180) from [`super::grid::month_grid`] and [`super::grid::busy_days`]: macOS's calendar
//! widget, the month's name in the accent, weekday initials, today on the accent disc, a dot
//! under each day with an event: compact in the desktop's small frame, regular in the center's
//! tile (quire's `MonthDensity::Auto`, Q190). Redrawn on the clock's minute tick.

use dioxus::prelude::*;
use jiff::tz::TimeZone;
use sill_settings::WeekNumbers;

use super::grid::{Step, busy_days, month_grid};
use super::shown::{Shown, grid_data};
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
    // The month's density is quire's to pick from the frame (`MonthDensity::Auto`): compact in
    // the desktop's small frame (Q190), regular in the notification center's large tile.
    let class = match host {
        WidgetHost::Tile => "sill-cal sill-cal-center",
        WidgetHost::Desktop => "sill-cal",
    };
    rsx! {
        div { class,
            ds::MonthGrid { data: grid_data(&grid, today), weeks, onstep }
        }
    }
}
