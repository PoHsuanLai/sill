//! The World Clock widget's provider: each zone in `widgets.world_clocks` as
//! quire's `ClockCity`, the city's time, day or night there, and under it two notes: its day
//! ("Today", or its date, "Sat 27 Sep", when that differs from here's) and how far it is from
//! here ("+7h"), a new entry on the clock's minute tick. quire's
//! `WorldClockWidget` draws them (dials on the desktop, digits in the tile, up to
//! `sill_shell_kit::widget::MAX_CITIES`). A zone jiff does not know is left out.

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use sill_shell_kit::clock::kind::{ClockTime, DayPhase, Seconds};
use sill_shell_kit::widget::clock::{ClockCity, ClockEntry};

/// An IANA zone name: `Europe/London`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ZoneName(pub String);

/// The calendar day in a zone against here's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DayThere {
    /// The same date as here.
    Today,
    /// Another date than here's: the one it is there.
    On(Date),
}

impl DayThere {
    /// `date` there against `here`'s date.
    pub fn of(there: Date, here: Date) -> DayThere {
        match there == here {
            true => DayThere::Today,
            false => DayThere::On(there),
        }
    }

    /// The line under the city: `Today`, or the date as `Sat 27 Sep`.
    pub fn text(&self) -> String {
        match self {
            DayThere::Today => "Today".to_owned(),
            DayThere::On(date) => date.strftime("%a %-d %b").to_string(),
        }
    }
}

/// One row.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClockRow {
    /// The zone's last segment, underscores as spaces: `New York`.
    pub city: String,
    /// The wall-clock time there, to the minute.
    pub time: ClockTime,
    /// Day or night there, which tints the dial.
    pub phase: DayPhase,
    /// Its date when it is not today's here.
    pub day: DayThere,
    /// `+7h`, `-5h30`, `same time`.
    pub offset: String,
}

/// The first hour a dial reads as day, and the first it reads as night again (macOS turns its
/// world clock's face dark from 18:00 to 06:00).
const DAY_HOURS: std::ops::Range<i8> = 6..18;

/// Day or night at `hour` (0 to 23) there.
pub fn phase_of(hour: i8) -> DayPhase {
    match DAY_HOURS.contains(&hour) {
        true => DayPhase::Day,
        false => DayPhase::Night,
    }
}

/// The wall-clock time of `hour` and `minute` as jiff hands them, the seconds hidden.
pub(super) fn clock_time(hour: i8, minute: i8) -> ClockTime {
    let unsigned = |value: i8| u8::try_from(value).unwrap_or_default();
    ClockTime {
        hour: unsigned(hour),
        minute: unsigned(minute),
        second: Seconds::Hidden,
    }
}

/// `widgets.world_clocks`: `;`-separated, trimmed, empty entries skipped, duplicates kept once.
pub fn parse_zones(text: &str) -> Vec<ZoneName> {
    text.split(';')
        .map(str::trim)
        .filter(|zone| !zone.is_empty())
        .fold(Vec::new(), |mut zones, zone| {
            let zone = ZoneName(zone.to_owned());
            if !zones.contains(&zone) {
                zones.push(zone);
            }
            zones
        })
}

/// The rows for `zones` at `now`, relative to `here`; unknown zones left out.
pub fn clock_rows(now: Timestamp, zones: &[ZoneName], here: &TimeZone) -> Vec<ClockRow> {
    let local = now.to_zoned(here.clone());
    zones
        .iter()
        .filter_map(|zone| {
            let there = now.to_zoned(TimeZone::get(&zone.0).ok()?);
            let apart = there.offset().seconds() - local.offset().seconds();
            Some(ClockRow {
                city: city_of(zone),
                time: clock_time(there.hour(), there.minute()),
                phase: phase_of(there.hour()),
                day: DayThere::of(there.date(), local.date()),
                offset: offset_text(apart),
            })
        })
        .collect()
}

/// The zone's last segment, underscores as spaces.
fn city_of(zone: &ZoneName) -> String {
    zone.0
        .rsplit('/')
        .next()
        .unwrap_or(&zone.0)
        .replace('_', " ")
}

/// `+7h`, `-5h30`, `same time`.
pub(super) fn offset_text(seconds: i32) -> String {
    if seconds == 0 {
        return "same time".to_owned();
    }
    let sign = if seconds < 0 { '-' } else { '+' };
    let minutes = seconds.unsigned_abs() / 60;
    match (minutes / 60, minutes % 60) {
        (hours, 0) => format!("{sign}{hours}h"),
        (hours, rest) => format!("{sign}{hours}h{rest:02}"),
    }
}

/// The widget's entry for `rows`; none: a quiet line naming the key.
pub fn clock_entry(rows: Vec<ClockRow>) -> ClockEntry {
    match rows.is_empty() {
        true => ClockEntry::Absent("No cities: add them in widgets.world_clocks".to_owned()),
        false => ClockEntry::Cities(
            rows.into_iter()
                .map(|row| ClockCity {
                    name: row.city,
                    time: row.time,
                    phase: row.phase,
                    notes: vec![row.day.text(), row.offset],
                })
                .collect(),
        ),
    }
}
