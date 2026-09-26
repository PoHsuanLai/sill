//! The Up Next widget (M10 lane a): the next events (`sill_services::events::up_next` over
//! `calendar.up_next_days` and `calendar.up_next_max`), each a title and a time line; "No
//! upcoming events" when there are none, and a quiet line naming the source state when the
//! calendars are off or unreadable. Redrawn on the clock's minute tick.
//!
//! G291: `up_next_max` is a ceiling, not a promise. The tile's body is a fixed number of
//! pixels (`--widget-cell`, `--widget-gap`; quire's `WidgetFrame`), so asking for more rows
//! than it holds used to draw the last one half off the bottom, clipped by the card's
//! `overflow:hidden`. [`events_capacity`] caps the count to what the frame's size class holds
//! at the default tokens, measured once against a real render (`ds_native::Harness`, not
//! measured again at draw time) rather than guessed: a Small or Medium tile is one cell tall
//! either way (164 default; only a medium's width differs), so both hold the same two whole
//! rows; a Large tile, two cells tall, holds six.

use dioxus::prelude::*;
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use sill_services::events::{CalendarName, Days, up_next};
use sill_services::{CalEvent, EventTime, SourceStatus};

use super::{WidgetHost, WidgetKind, WidgetSize, size_of};
use crate::hooks::{use_agenda, use_clock, use_settings};

/// How many whole event rows the tile's body holds at `size`, measured against the default
/// tokens (a 164 px small cell, 16 px gap, 12 px card padding, the title row and `.sill-un-day`
/// column already drawn) rather than assumed: a Small and a Medium tile share one cell's
/// height, so both hold two rows before a third would clip; a Large tile, two cells and a gap
/// tall, holds six.
fn events_capacity(size: WidgetSize) -> usize {
    match size {
        WidgetSize::Small | WidgetSize::Medium => 2,
        WidgetSize::Large => 6,
    }
}

/// This widget's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("../style/widget_up_next.css");

/// What one event's row says.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventLines {
    /// The title, "Untitled" when empty.
    pub title: String,
    /// `All day`, `Now`, `10:00 – 11:30`, `Tomorrow 09:00`, `Mon 09:00`: 24-hour, like the bar
    /// clock (a 12-hour form has no settings key yet).
    pub when: String,
}

/// `event`'s lines as seen at `now` in `zone`.
pub fn event_lines(event: &CalEvent, now: Timestamp, zone: &TimeZone) -> EventLines {
    let title = match event.title.trim().is_empty() {
        true => "Untitled".to_owned(),
        false => event.title.clone(),
    };
    let today = now.to_zoned(zone.clone()).date();
    let when = match event.when {
        EventTime::AllDay { first, .. } if first <= today => "All day".to_owned(),
        EventTime::AllDay { first, .. } => format!("{}, all day", day_word(first, today)),
        EventTime::Timed { start, end } if start <= now && now < end => "Now".to_owned(),
        EventTime::Timed { start, end } => {
            let from = start.to_zoned(zone.clone());
            let hm = |at: &jiff::Zoned| format!("{:02}:{:02}", at.hour(), at.minute());
            match (from.date() == today, end > start) {
                (true, true) => format!("{} – {}", hm(&from), hm(&end.to_zoned(zone.clone()))),
                (true, false) => hm(&from),
                (false, _) => format!("{} {}", day_word(from.date(), today), hm(&from)),
            }
        }
    };
    EventLines { title, when }
}

/// A later day as Up Next names it: `Tomorrow`, a weekday within the week, else `5 Oct`.
fn day_word(day: Date, today: Date) -> String {
    let ahead = today.until(day).map_or(0, |span| span.get_days());
    match ahead {
        1 => "Tomorrow".to_owned(),
        2..=6 => {
            WEEKDAYS[usize::from(day.weekday().to_monday_zero_offset().unsigned_abs())].to_owned()
        }
        _ => format!(
            "{} {}",
            day.day(),
            MONTHS[usize::from((day.month() - 1).unsigned_abs()) % 12]
        ),
    }
}

const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// The calendar colour's class: one of quire's eight person swatches (`--c-person-N`), picked by
/// the calendar's name, so a calendar keeps its colour everywhere.
fn hue_class(calendar: &CalendarName) -> String {
    let hash = calendar.0.bytes().fold(2_166_136_261_u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(16_777_619)
    });
    format!("sill-un-bar sill-un-hue-{}", hash % 8 + 1)
}

/// What the widget says instead of, or under, its events.
fn quiet_line(status: &SourceStatus) -> Option<String> {
    match status {
        SourceStatus::Off => Some("Calendars are off".to_owned()),
        SourceStatus::NoCalendars => Some("No calendars found".to_owned()),
        SourceStatus::Read { problems, .. } => match problems.len() {
            0 => None,
            1 => Some("1 calendar file could not be read".to_owned()),
            n => Some(format!("{n} calendar files could not be read")),
        },
    }
}

/// The next events.
#[component]
pub fn UpNextWidget(host: WidgetHost) -> Element {
    let _minute = use_clock()();
    let agenda = use_agenda()();
    let calendar = use_settings()().calendar;
    let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
    let now = Timestamp::now();
    let today = now.to_zoned(zone.clone()).date();
    let max =
        usize::from(calendar.up_next_max.0).min(events_capacity(size_of(WidgetKind::UpNext, host)));
    let next = up_next(
        &agenda.events,
        now,
        &zone,
        Days(calendar.up_next_days.0),
        max,
    );
    let rows: Vec<(String, EventLines)> = next
        .iter()
        .map(|event| (hue_class(&event.calendar), event_lines(event, now, &zone)))
        .collect();
    let quiet = quiet_line(&agenda.status);
    let host_class = match host {
        WidgetHost::Tile => "sill-un sill-un-center",
        WidgetHost::Desktop => "sill-un sill-un-desktop",
    };
    let weekday =
        WEEKDAY_NAMES[usize::from(today.weekday().to_monday_zero_offset().unsigned_abs())];
    rsx! {
        div { class: host_class,
            div { class: "sill-un-day",
                span { class: "sill-un-weekday", {weekday} }
                span { class: "sill-un-date", {today.day().to_string()} }
            }
            div { class: "sill-un-list",
                if rows.is_empty() && quiet.is_none() {
                    span { class: "sill-un-empty", "No upcoming events" }
                }
                for (hue, lines) in rows {
                    div { class: "sill-un-row",
                        span { class: hue }
                        div { class: "sill-un-text",
                            span { class: "sill-un-title ds-truncate", {lines.title} }
                            span { class: "sill-un-when", {lines.when} }
                        }
                    }
                }
                if let Some(quiet) = quiet {
                    span { class: "sill-un-quiet", {quiet} }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use jiff::tz::TimeZone;
    use sill_services::events::{CalendarName, EventUid};
    use sill_services::{CalEvent, EventTime};

    use super::{EventLines, WidgetSize, event_lines, events_capacity};

    #[test]
    fn a_size_holds_as_many_rows_as_its_cell_measured_out() {
        let cases = [
            (WidgetSize::Small, 2),
            (WidgetSize::Medium, 2),
            (WidgetSize::Large, 6),
        ];
        for (size, want) in cases {
            assert_eq!(events_capacity(size), want, "{size:?}");
        }
    }

    #[test]
    fn an_events_time_line_reads_from_now() {
        let utc = TimeZone::UTC;
        let at = |d, h, m| {
            date(2026, 9, d)
                .at(h, m, 0, 0)
                .to_zoned(utc.clone())
                .map(|z| z.timestamp())
                .unwrap_or_else(|e| panic!("{e}"))
        };
        let event = |title: &str, when| CalEvent {
            uid: EventUid("x".to_owned()),
            title: title.to_owned(),
            when,
            location: None,
            calendar: CalendarName("std".to_owned()),
        };
        let now = at(26, 10, 0);
        let lines = |title: &str, when: &str| EventLines {
            title: title.to_owned(),
            when: when.to_owned(),
        };
        let cases = [
            (
                event(
                    "Standup",
                    EventTime::Timed {
                        start: at(26, 10, 30),
                        end: at(26, 11, 0),
                    },
                ),
                lines("Standup", "10:30 – 11:00"),
            ),
            (
                event(
                    "",
                    EventTime::Timed {
                        start: at(26, 9, 30),
                        end: at(26, 11, 0),
                    },
                ),
                lines("Untitled", "Now"),
            ),
            (
                event(
                    "Dentist",
                    EventTime::Timed {
                        start: at(27, 9, 0),
                        end: at(27, 10, 0),
                    },
                ),
                lines("Dentist", "Tomorrow 09:00"),
            ),
            (
                event(
                    "Holiday",
                    EventTime::AllDay {
                        first: date(2026, 9, 26),
                        last: date(2026, 9, 26),
                    },
                ),
                lines("Holiday", "All day"),
            ),
            (
                event(
                    "Trip",
                    EventTime::Timed {
                        start: at(28, 9, 0),
                        end: at(28, 10, 0),
                    },
                ),
                lines("Trip", "Mon 09:00"),
            ),
        ];
        for (event, want) in cases {
            assert_eq!(event_lines(&event, now, &utc), want, "{}", event.title);
        }
    }
}
