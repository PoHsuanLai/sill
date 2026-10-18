//! The Up Next widget's provider: the next events (`sill_model::events::up_next`
//! over `calendar.up_next_days` and `calendar.up_next_max`), each a title and a time line, as
//! [`UpNextEntry`]; "No upcoming events" when there are none, and a quiet line naming the source
//! state when the calendars are off or unreadable. A new entry on the clock's minute tick. The
//! widget itself ([`UpNext`], `widget.rs`) is a `sill_shell_kit::prelude::Widget` of kind `sill.up-next`. The
//! same agenda feeds the month widget's events ([`today_events`]), in the same colours.
//!
//! `up_next_max` is a ceiling, not a promise. The card's body is a fixed number of pixels
//! (`--widget-cell`, `--widget-gap`; quire's card), so asking for more rows than it holds would
//! draw the last one half off the bottom, clipped by the card's `overflow:hidden`. The view draws
//! at most [`EVENT_ROWS`], measured once against a real render (`ds_harness::Harness`) rather
//! than guessed.

use ds::prelude::*;
use ds::style::tokens::label_hue::LabelHue;
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use sill_model::events::{Agenda, CalEvent, CalendarName, Days, EventTime, SourceStatus, up_next};
use sill_shell_kit::widget::calendar::EventLine;

use super::widget::{DayLine, UpNextEntry, UpNextFace, UpNextRow};
use crate::widgets::calendar::shown::weekday_name;

/// How many whole event rows the card's body holds at its one size (Medium: one 164 px cell
/// tall, 16 px gap, quire's card inset, the title row and `.sill-un-day` column already drawn):
/// two before a third would clip.
pub const EVENT_ROWS: usize = 2;

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
/// first; quire caps the list at `sill_shell_kit::widget::LARGE_EVENTS`): the same agenda and lines Up Next
/// draws.
pub fn today_events(agenda: &Agenda, now: Timestamp, zone: &TimeZone) -> Vec<EventLine> {
    up_next(
        &agenda.events,
        now,
        zone,
        Days(0),
        sill_shell_kit::widget::LARGE_EVENTS,
    )
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
