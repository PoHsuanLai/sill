//! Formatting the bar clock: pure over a civil date-time, so the time zone and the clock are
//! the caller's (CONVENTIONS §6).

use std::time::Duration;

use jiff::civil::{DateTime, Time, Weekday};
use serde::{Deserialize, Serialize};

/// The clock's text as the bar shows it: `Thu 24 Sep 14:05`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ClockText {
    pub text: String,
}

/// `now` as `<weekday> <day> <month> <HH>:<MM>`, English abbreviations, 24-hour. Locale and a
/// 12-hour form have no settings key in design/22 yet (FINDINGS.md).
pub fn format_clock(now: DateTime) -> ClockText {
    ClockText {
        text: format!(
            "{} {} {} {:02}:{:02}",
            weekday(now.weekday()),
            now.day(),
            month(now.month()),
            now.hour(),
            now.minute()
        ),
    }
}

/// How long from `now` until the next minute starts; a full minute when `now` is exactly on
/// one.
pub fn until_next_minute(now: Time) -> Duration {
    let into_minute = Duration::from_secs(u64::from(now.second().unsigned_abs()))
        + Duration::from_nanos(u64::from(now.subsec_nanosecond().unsigned_abs()));
    Duration::from_secs(60).saturating_sub(into_minute)
}

fn weekday(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => "Mon",
        Weekday::Tuesday => "Tue",
        Weekday::Wednesday => "Wed",
        Weekday::Thursday => "Thu",
        Weekday::Friday => "Fri",
        Weekday::Saturday => "Sat",
        Weekday::Sunday => "Sun",
    }
}

fn month(month: i8) -> &'static str {
    const NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    usize::try_from(month - 1)
        .ok()
        .and_then(|i| NAMES.get(i))
        .copied()
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::{format_clock, until_next_minute};
    use jiff::civil::{date, time};
    use std::time::Duration;

    #[test]
    fn the_clock_reads_weekday_day_month_and_24_hour_time() {
        let cases = [
            (date(2026, 9, 24).at(14, 5, 0, 0), "Thu 24 Sep 14:05"),
            (
                date(2026, 9, 24).at(0, 0, 59, 999_999_999),
                "Thu 24 Sep 00:00",
            ),
            (date(2026, 1, 1).at(9, 30, 0, 0), "Thu 1 Jan 09:30"),
            (date(2027, 12, 31).at(23, 59, 0, 0), "Fri 31 Dec 23:59"),
            (date(2028, 2, 29).at(12, 0, 0, 0), "Tue 29 Feb 12:00"),
            (date(2026, 6, 7).at(7, 7, 7, 0), "Sun 7 Jun 07:07"),
        ];
        for (now, want) in cases {
            assert_eq!(format_clock(now).text, want, "{now}");
        }
    }

    #[test]
    fn the_next_minute_is_what_remains_of_this_one() {
        let cases = [
            (time(14, 5, 0, 0), Duration::from_secs(60)),
            (time(14, 5, 30, 0), Duration::from_secs(30)),
            (time(14, 5, 59, 0), Duration::from_secs(1)),
            (time(14, 5, 59, 999_000_000), Duration::from_millis(1)),
            (time(14, 5, 12, 250_000_000), Duration::from_millis(47_750)),
            (time(23, 59, 59, 999_999_999), Duration::from_nanos(1)),
        ];
        for (now, want) in cases {
            assert_eq!(until_next_minute(now), want, "{now}");
        }
    }
}
