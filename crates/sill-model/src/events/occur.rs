//! One event's shape (whole days, or a time of day in a zone, and how long it lasts), its
//! `EXDATE`s, and a window's overlap: what [`super::rule::occurrences`] puts on each day
//! [`super::rule_expand::expand`] finds.

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};

use super::ics::{IcsEnd, IcsEvent, IcsTime};
use super::rule::RuleEnd;
use super::rule_expand::Bounds;
use super::{EventTime, Window};

/// When each occurrence happens, less its day.
#[derive(Debug, Clone)]
pub(super) enum Shape {
    /// All day, `days` days long (at least one).
    Days { first: Date, days: i64 },
    /// At `at`'s time of day in `zone`, `seconds` long (never negative).
    Clock {
        at: DateTime,
        zone: TimeZone,
        seconds: i64,
    },
}

/// How far a zone can move a civil day from the viewer's: the expansion's slack at each end.
const SLACK_DAYS: i64 = 2;

impl Shape {
    /// `event`'s shape, a floating time (or a zone jiff does not know) in `viewer`.
    pub fn of(event: &IcsEvent, viewer: &TimeZone) -> Shape {
        match civil_of(&event.start, viewer) {
            (Moment::Day(first), _) => Shape::Days {
                first,
                days: whole_days(first, &event.end, viewer),
            },
            (Moment::Clock(at), zone) => {
                let start = instant(at, &zone);
                let seconds = match &event.end {
                    IcsEnd::At(end) => match (start, resolve(end, viewer)) {
                        (Some(start), Some(end)) => end.as_second() - start.as_second(),
                        _ => 0,
                    },
                    IcsEnd::Lasts(seconds) => *seconds,
                    IcsEnd::Unsaid => 0,
                };
                Shape::Clock {
                    at,
                    zone,
                    seconds: seconds.max(0),
                }
            }
        }
    }

    /// The first occurrence's civil day (in the event's own zone).
    pub fn first_day(&self) -> Date {
        match self {
            Shape::Days { first, .. } => *first,
            Shape::Clock { at, .. } => at.date(),
        }
    }

    /// The days an expansion keeps for `window`: from as far back as an occurrence can start
    /// and still reach into it, to a little past its end.
    pub fn bounds(&self, window: Window) -> Bounds {
        let reach = match self {
            Shape::Days { days, .. } => *days,
            Shape::Clock { seconds, .. } => seconds / 86_400 + 1,
        };
        Bounds {
            keep_from: window.from.saturating_sub((reach + SLACK_DAYS).days()),
            stop_after: window.until.saturating_add(SLACK_DAYS.days()),
        }
    }

    /// The occurrence on `day`; `None` when its time does not exist there.
    pub fn at(&self, day: Date) -> Option<EventTime> {
        match self {
            Shape::Days { days, .. } => Some(EventTime::AllDay {
                first: day,
                last: day.checked_add((days - 1).days()).ok()?,
            }),
            Shape::Clock { at, zone, seconds } => {
                let start = instant(day.to_datetime(at.time()), zone)?;
                let end = start.checked_add(seconds.seconds()).ok()?;
                Some(EventTime::Timed { start, end })
            }
        }
    }

    /// Whether the occurrence on `day` is past `end`'s `UNTIL`.
    pub fn past_until(&self, day: Date, end: &RuleEnd, viewer: &TimeZone) -> bool {
        let RuleEnd::Until(until) = end else {
            return false;
        };
        let past = match (self, until) {
            (Shape::Days { .. }, until) => Some(day > civil_of(until, viewer).0.day()),
            (Shape::Clock { .. }, IcsTime::Date(last)) => Some(day > *last),
            (Shape::Clock { at, zone, .. }, until) => instant(day.to_datetime(at.time()), zone)
                .zip(resolve(until, viewer))
                .map(|(start, until)| start > until),
        };
        past == Some(true)
    }
}

/// The `EXDATE`s of one event: days (every occurrence that day) and instants.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct Exclusions {
    days: Vec<Date>,
    instants: Vec<Timestamp>,
}

impl Exclusions {
    /// `exdates` resolved against `viewer`.
    pub fn of(exdates: &[IcsTime], viewer: &TimeZone) -> Exclusions {
        let mut out = Exclusions::default();
        for time in exdates {
            match time {
                IcsTime::Date(day) => out.days.push(*day),
                other => out.instants.extend(resolve(other, viewer)),
            }
        }
        out
    }

    /// Whether `when` is excluded: an all-day occurrence by its first day (an instant's day in
    /// `viewer` counts too), a timed one by its start or its day.
    pub fn covers(&self, when: &EventTime, viewer: &TimeZone) -> bool {
        match when {
            EventTime::AllDay { first, .. } => {
                self.days.contains(first)
                    || self
                        .instants
                        .iter()
                        .any(|at| at.to_zoned(viewer.clone()).date() == *first)
            }
            EventTime::Timed { start, .. } => {
                self.instants.contains(start)
                    || self.days.contains(&start.to_zoned(viewer.clone()).date())
            }
        }
    }
}

/// Whether `when` overlaps `window` (its days in `viewer`); a moment counts where it falls.
pub(super) fn overlaps(when: &EventTime, window: Window, viewer: &TimeZone) -> bool {
    match when {
        EventTime::AllDay { first, last } => *first < window.until && *last >= window.from,
        EventTime::Timed { start, end } => {
            let from = instant(
                window.from.to_datetime(jiff::civil::Time::midnight()),
                viewer,
            );
            let until = instant(
                window.until.to_datetime(jiff::civil::Time::midnight()),
                viewer,
            );
            match (from, until) {
                (Some(from), Some(until)) => {
                    *start < until && (*end > from || (*end == *start && *start >= from))
                }
                _ => false,
            }
        }
    }
}

/// A date or a time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Moment {
    Day(Date),
    Clock(DateTime),
}

impl Moment {
    fn day(self) -> Date {
        match self {
            Moment::Day(day) => day,
            Moment::Clock(at) => at.date(),
        }
    }
}

/// `time` as civil, with the zone it is civil in (UTC for a UTC time).
fn civil_of(time: &IcsTime, viewer: &TimeZone) -> (Moment, TimeZone) {
    match time {
        IcsTime::Date(day) => (Moment::Day(*day), viewer.clone()),
        IcsTime::Utc(at) => (
            Moment::Clock(at.to_zoned(TimeZone::UTC).datetime()),
            TimeZone::UTC,
        ),
        IcsTime::Zoned { at, zone } => (Moment::Clock(*at), zone_named(zone, viewer)),
        IcsTime::Floating(at) => (Moment::Clock(*at), viewer.clone()),
    }
}

/// `time` as an instant: a date at its midnight in `viewer`.
pub(super) fn resolve(time: &IcsTime, viewer: &TimeZone) -> Option<Timestamp> {
    match civil_of(time, viewer) {
        (Moment::Day(day), zone) => instant(day.to_datetime(jiff::civil::Time::midnight()), &zone),
        (Moment::Clock(at), zone) => instant(at, &zone),
    }
}

/// `at` in `zone`; a time a clock change skips is moved forward, as RFC 5545 says.
fn instant(at: DateTime, zone: &TimeZone) -> Option<Timestamp> {
    zone.to_ambiguous_zoned(at)
        .compatible()
        .ok()
        .map(|zoned| zoned.timestamp())
}

/// The zone `name` names: an IANA name, or one with a vendor prefix
/// (`/mozilla.org/20050126_1/Europe/Berlin`); else `viewer` (a Windows name, say).
pub(super) fn zone_named(name: &str, viewer: &TimeZone) -> TimeZone {
    let parts: Vec<&str> = name.trim().trim_matches('/').split('/').collect();
    (0..parts.len())
        .find_map(|skip| TimeZone::get(&parts[skip..].join("/")).ok())
        .unwrap_or_else(|| viewer.clone())
}

/// How many days an all-day event from `first` lasts (at least one).
fn whole_days(first: Date, end: &IcsEnd, viewer: &TimeZone) -> i64 {
    let days = match end {
        IcsEnd::At(end) => {
            let last = civil_of(end, viewer).0.day();
            first
                .until(last)
                .ok()
                .map_or(1, |span| i64::from(span.get_days()))
        }
        IcsEnd::Lasts(seconds) => (seconds + 86_399) / 86_400,
        IcsEnd::Unsaid => 1,
    };
    days.max(1)
}

#[cfg(test)]
mod tests;
