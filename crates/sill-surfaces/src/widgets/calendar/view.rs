//! The Calendar widget's view: reads `use_agenda` (today, the events) and the `calendar`
//! settings, keeps the month shown in a signal (today's month on mount, stepped by the previous
//! and next buttons, back to today's month when the day turns), and draws
//! [`super::grid::month_grid`] with [`super::grid::busy_days`], macOS's calendar widget: the
//! month's name in the accent, weekday initials, today on the accent disc, a dot under each day
//! with an event. Text laid out with quire's tokens until quire's `MonthGrid` (Q180).

use dioxus::prelude::*;
use ds::{Icon, IconButton, IconButtonVariant};
use jiff::tz::TimeZone;
use sill_settings::WeekNumbers;

use super::grid::{DayMark, DayPlace, Eventful, GridDay, GridWeek, Step, busy_days, month_grid};
use super::shown::{Shown, initial, month_title};
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
    let numbers = calendar.week_numbers;
    let host_class = match host {
        WidgetHost::Center => "sill-cal sill-cal-center",
        WidgetHost::Desktop => "sill-cal sill-cal-desktop",
    };
    let step = move |to: Step| move |_| shown.set(shown().stepped(today, to));
    rsx! {
        div { class: host_class,
            div { class: "sill-cal-head",
                span { class: "sill-cal-title", {month_title(month, today)} }
                div { class: "sill-cal-steps",
                    IconButton {
                        variant: IconButtonVariant::Tool,
                        icon: Icon::ChevronLeft,
                        label: "Previous month".to_owned(),
                        onclick: step(Step::Previous),
                    }
                    IconButton {
                        variant: IconButtonVariant::Tool,
                        icon: Icon::ChevronRight,
                        label: "Next month".to_owned(),
                        onclick: step(Step::Next),
                    }
                }
            }
            div { class: "sill-cal-row sill-cal-heads",
                if numbers == WeekNumbers::Show {
                    span { class: "sill-cal-week" }
                }
                for day in grid.weekdays {
                    span { class: "sill-cal-cell sill-cal-weekday", {initial(day)} }
                }
            }
            for week in grid.weeks {
                Week { week, numbers }
            }
        }
    }
}

/// One row: its ISO week when shown, then its seven days.
#[component]
fn Week(week: GridWeek, numbers: WeekNumbers) -> Element {
    rsx! {
        div { class: "sill-cal-row",
            if numbers == WeekNumbers::Show {
                span { class: "sill-cal-week", {week.number.0.to_string()} }
            }
            for day in week.days {
                Day { day }
            }
        }
    }
}

/// One day's cell: its number (on the accent disc today), quieter outside the month, and a dot
/// when it has an event.
#[component]
fn Day(day: GridDay) -> Element {
    let place = match day.place {
        DayPlace::InMonth => "sill-cal-in",
        DayPlace::Before | DayPlace::After => "sill-cal-out",
    };
    let mark = match day.mark {
        DayMark::Today => "sill-cal-num sill-cal-today",
        DayMark::Plain => "sill-cal-num",
    };
    let dot = match day.events {
        Eventful::Busy => "sill-cal-dot sill-cal-busy",
        Eventful::Free => "sill-cal-dot",
    };
    rsx! {
        span { class: "sill-cal-cell sill-cal-day {place}",
            span { class: mark, {day.date.day().to_string()} }
            span { class: dot }
        }
    }
}
