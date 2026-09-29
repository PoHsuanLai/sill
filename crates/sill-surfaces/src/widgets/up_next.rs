//! The Up Next widget's provider: the next events (`sill_model::events::up_next`
//! over `calendar.up_next_days` and `calendar.up_next_max`), each a title and a time line, as
//! [`UpNextEntry`]; "No upcoming events" when there are none, and a quiet line naming the source
//! state when the calendars are off or unreadable. A new entry on the clock's minute tick. The
//! widget itself ([`UpNext`], `up_next_view.rs`) is a `ds::Widget` of kind `sill.up-next`. The
//! same agenda feeds the month widget's events ([`today_events`]), in the same colours.
//!
//! `up_next_max` is a ceiling, not a promise. The card's body is a fixed number of pixels
//! (`--widget-cell`, `--widget-gap`; quire's card), so asking for more rows than it holds would
//! draw the last one half off the bottom, clipped by the card's `overflow:hidden`. The view draws
//! at most [`EVENT_ROWS`], measured once against a real render (`ds_native::Harness`) rather
//! than guessed.

use dioxus::prelude::*;
use ds::Word;
use ds::{EventLine, LabelHue, WidgetCard};
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use sill_model::events::{Agenda, CalEvent, CalendarName, Days, EventTime, SourceStatus, up_next};

pub use super::up_next_view::{DayLine, UpNext, UpNextEntry, UpNextFace, UpNextRow};
use super::{Place, WidgetKind, widget_element_id};
use crate::hooks::{use_service, use_settings};
use crate::widgets::use_widget_wake;
use sill_services::{ClockService, EventsService};

/// How many whole event rows the card's body holds at its one size (Medium: one 164 px cell
/// tall, 16 px gap, quire's card inset, the title row and `.sill-un-day` column already drawn):
/// two before a third would clip.
pub const EVENT_ROWS: usize = 2;

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

/// A calendar's colour: one of quire's label hues, picked by the calendar's name, so a calendar
/// keeps its colour everywhere (Up Next's bar, the month widget's events).
pub fn calendar_hue(calendar: &CalendarName) -> LabelHue {
    let hash = calendar.0.bytes().fold(2_166_136_261_u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(16_777_619)
    });
    let at = usize::try_from(hash).unwrap_or(0) % LabelHue::ALL.len();
    LabelHue::ALL[at]
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

/// The weekday's name.
fn weekday_name(day: Date) -> &'static str {
    WEEKDAY_NAMES[usize::from(day.weekday().to_monday_zero_offset().unsigned_abs())]
}

/// The widget's entry: today, then up to `max` next events of `agenda` over `ahead` days as
/// seen at `now` in `zone`.
pub fn up_next_entry(
    agenda: &Agenda,
    now: Timestamp,
    zone: &TimeZone,
    ahead: Days,
    max: usize,
) -> UpNextEntry {
    let today = now.to_zoned(zone.clone()).date();
    let rows = up_next(&agenda.events, now, zone, ahead, max)
        .iter()
        .map(|event| {
            let lines = event_lines(event, now, zone);
            UpNextRow {
                hue: calendar_hue(&event.calendar),
                title: lines.title,
                when: lines.when,
            }
        })
        .collect();
    UpNextEntry::Agenda(UpNextFace {
        day: DayLine {
            weekday: weekday_name(today).to_owned(),
            date: u8::try_from(today.day()).unwrap_or(1),
        },
        rows,
        quiet: quiet_line(&agenda.status),
    })
}

/// Today's events still to come, as the month widget lists them (its Medium card shows the
/// first; quire caps the list at `ds::widget::LARGE_EVENTS`): the same agenda and lines Up Next
/// draws.
pub fn today_events(agenda: &Agenda, now: Timestamp, zone: &TimeZone) -> Vec<EventLine> {
    up_next(&agenda.events, now, zone, Days(0), ds::widget::LARGE_EVENTS)
        .iter()
        .map(|event| {
            let lines = event_lines(event, now, zone);
            EventLine {
                time: lines.when,
                title: lines.title,
                hue: calendar_hue(&event.calendar),
            }
        })
        .collect()
}

/// The next events on quire's card, as many as its body holds whole.
#[component]
pub fn UpNextCard(place: Place) -> Element {
    let _minute = use_service::<ClockService>()();
    let agenda = use_service::<EventsService>()();
    let calendar = use_settings()().calendar;
    let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
    let max = usize::from(calendar.up_next_max.0).min(EVENT_ROWS);
    let days = Days(calendar.up_next_days.0);
    let entry = up_next_entry(&agenda, Timestamp::now(), &zone, days, max);
    rsx! {
        WidgetCard {
            widget: UpNext,
            timeline: ds::Timeline::now(entry),
            size: place.size,
            host: place.host,
            wake: use_widget_wake(),
            id: widget_element_id(WidgetKind::UpNext),
            lift: place.lift,
            presence: place.presence,
            on_gone: place.on_gone,
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use jiff::tz::TimeZone;
    use sill_model::events::{CalEvent, EventTime};
    use sill_model::events::{CalendarName, EventUid};

    use super::{EventLines, calendar_hue, event_lines};
    use ds::LabelHue;

    #[test]
    fn a_calendar_keeps_one_label_hue_by_its_name() {
        // FNV-1a of the name, modulo quire's five hues (`dev/accept-calendar.sh` computes the same).
        let cases = [
            ("work", LabelHue::Violet),
            ("team", LabelHue::Green),
            ("personal", LabelHue::Amber),
        ];
        for (name, want) in cases {
            assert_eq!(calendar_hue(&CalendarName(name.to_owned())), want, "{name}");
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
