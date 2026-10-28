//! iCalendar values: dates, date-times, durations and text (RFC 5545 §3.3).

use jiff::civil::{Date, DateTime, Time};
use jiff::tz::TimeZone;

use super::ics::IcsTime;
use super::ics_line::ContentLine;

/// Why an iCalendar value could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum ValueError {
    #[error("not a date: {0:?}")]
    Date(String),
    #[error("not a date-time: {0:?}")]
    DateTime(String),
    #[error("not a duration: {0:?}")]
    Duration(String),
    #[error("{value:?}: {error}")]
    Zone { value: String, error: String },
}

/// A `DTSTART`-like property's one value: `VALUE=DATE` or eight digits is a date; a date-time
/// ending in `Z` is UTC, one with `TZID` zoned, any other floating.
pub(super) fn parse_time(content: &ContentLine) -> Result<IcsTime, ValueError> {
    time_of(content, content.value.trim())
}

/// An `EXDATE`'s comma-separated values, each as [`parse_time`] reads one.
pub(super) fn parse_times(content: &ContentLine) -> Result<Vec<IcsTime>, ValueError> {
    content
        .value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| time_of(content, value))
        .collect()
}

fn time_of(content: &ContentLine, value: &str) -> Result<IcsTime, ValueError> {
    let is_date = content
        .param("VALUE")
        .is_some_and(|kind| kind.eq_ignore_ascii_case("DATE"))
        || value.len() == 8;
    if is_date {
        return parse_date(value).map(IcsTime::Date);
    }
    // `utc`: the date-time ended in `Z`.
    let (civil, utc) = match value.strip_suffix(['Z', 'z']) {
        Some(civil) => (civil, true),
        None => (value, false),
    };
    let at = parse_date_time(civil)?;
    match (utc, content.param("TZID")) {
        (true, _) => at
            .to_zoned(TimeZone::UTC)
            .map(|zoned| IcsTime::Utc(zoned.timestamp()))
            .map_err(|error| ValueError::Zone {
                value: value.to_owned(),
                error: error.to_string(),
            }),
        (false, Some(zone)) => Ok(IcsTime::Zoned {
            at,
            zone: zone.to_owned(),
        }),
        (false, None) => Ok(IcsTime::Floating(at)),
    }
}

/// `20260927`.
pub(super) fn parse_date(value: &str) -> Result<Date, ValueError> {
    let bad = || ValueError::Date(value.to_owned());
    let digits = |from: usize, to: usize| -> Result<i32, ValueError> {
        value
            .get(from..to)
            .filter(|part| part.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|part| part.parse().ok())
            .ok_or_else(bad)
    };
    if value.len() != 8 {
        return Err(bad());
    }
    let year = i16::try_from(digits(0, 4)?).map_err(|_| bad())?;
    let month = i8::try_from(digits(4, 6)?).map_err(|_| bad())?;
    let day = i8::try_from(digits(6, 8)?).map_err(|_| bad())?;
    Date::new(year, month, day).map_err(|_| bad())
}

/// `20260927T090000` (a leap second read as :59).
pub(super) fn parse_date_time(value: &str) -> Result<DateTime, ValueError> {
    let bad = || ValueError::DateTime(value.to_owned());
    let (day, clock) = value.split_once(['T', 't']).ok_or_else(bad)?;
    let date = parse_date(day).map_err(|_| bad())?;
    let part = |from: usize| -> Result<i8, ValueError> {
        clock
            .get(from..from + 2)
            .filter(|part| part.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|part| part.parse().ok())
            .ok_or_else(bad)
    };
    if clock.len() != 6 {
        return Err(bad());
    }
    let time = Time::new(part(0)?, part(2)?, part(4)?.min(59), 0).map_err(|_| bad())?;
    Ok(date.to_datetime(time))
}

/// `DURATION`: `[+-]P` then weeks, or days and a `T` part of hours, minutes and seconds, in
/// seconds (a day as 86 400).
pub(super) fn parse_duration(value: &str) -> Result<i64, ValueError> {
    let bad = || ValueError::Duration(value.to_owned());
    let value = value.trim();
    let (sign, rest) = match value.as_bytes().first() {
        Some(b'-') => (-1, &value[1..]),
        Some(b'+') => (1, &value[1..]),
        _ => (1, value),
    };
    let rest = rest
        .strip_prefix(['P', 'p'])
        .filter(|rest| !rest.is_empty())
        .ok_or_else(bad)?;
    let mut total: i64 = 0;
    let mut number = String::new();
    let mut in_time = Clock::Date;
    for c in rest.chars() {
        let unit: i64 = match (c.to_ascii_uppercase(), in_time) {
            (d, _) if d.is_ascii_digit() => {
                number.push(d);
                continue;
            }
            ('T', Clock::Date) => {
                in_time = Clock::Time;
                continue;
            }
            ('W', Clock::Date) => 7 * 86_400,
            ('D', Clock::Date) => 86_400,
            ('H', Clock::Time) => 3_600,
            ('M', Clock::Time) => 60,
            ('S', Clock::Time) => 1,
            _ => return Err(bad()),
        };
        let count: i64 = number.parse().map_err(|_| bad())?;
        total = total.saturating_add(count.saturating_mul(unit));
        number.clear();
    }
    match number.is_empty() {
        true => Ok(sign * total),
        false => Err(bad()),
    }
}

/// Before or after a duration's `T`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Clock {
    Date,
    Time,
}

/// A text value unescaped: `\n` and `\N` a newline, `\,` `\;` `\\` themselves.
pub(super) fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        match (c, c == '\\') {
            (_, true) => match chars.next() {
                Some('n' | 'N') => out.push('\n'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            },
            (c, false) => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::{parse_date_time, parse_duration, unescape};

    #[test]
    fn durations_read_in_seconds() {
        let cases = [
            ("PT1H30M", Ok(5400)),
            ("P1DT2H", Ok(93_600)),
            ("P2W", Ok(1_209_600)),
            ("-PT15M", Ok(-900)),
            ("PT0S", Ok(0)),
            ("P", Err(())),
            ("1H", Err(())),
            ("PT1", Err(())),
            ("P1H", Err(())),
        ];
        for (text, want) in cases {
            assert_eq!(parse_duration(text).map_err(|_| ()), want, "{text:?}");
        }
    }

    #[test]
    fn date_times_and_text() {
        assert_eq!(
            parse_date_time("20260927T093060"),
            Ok(date(2026, 9, 27).at(9, 30, 59, 0))
        );
        assert!(parse_date_time("20260927T0930").is_err());
        assert!(parse_date_time("20261327T093000").is_err());
        assert_eq!(unescape(r"a\, b\; c\\d\ne"), "a, b; c\\d\ne");
    }
}
