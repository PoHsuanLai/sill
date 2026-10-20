//! Formatting the bar clock: pure over a civil date-time, so the time zone and the clock are
//! the caller's (CONVENTIONS §6).

use std::time::Duration;

use jiff::civil::{DateTime, Time, Weekday};

use super::ClockHours;

/// `now` as `<weekday> <day> <month> <time>`, English abbreviations, the time in `hours`.
pub fn format_clock(now: DateTime, hours: ClockHours) -> String {
    format!(
        "{} {} {} {}",
        weekday(now.weekday()),
        now.day(),
        month(now.month()),
        time_of_day(now.time(), hours)
    )
}

/// `time` as `14:05` (24-hour) or `2:05 PM` (12-hour).
pub fn time_of_day(time: Time, hours: ClockHours) -> String {
    let digits = time_digits(time, hours);
    match hours {
        ClockHours::H24 => digits,
        ClockHours::H12 => format!("{digits} {}", day_part(time)),
    }
}

/// `time` as `14:05` (24-hour) or `2:05` (12-hour, without AM or PM: the lock screen's).
pub fn time_digits(time: Time, hours: ClockHours) -> String {
    match hours {
        ClockHours::H24 => format!("{:02}:{:02}", time.hour(), time.minute()),
        ClockHours::H12 => format!("{}:{:02}", (time.hour() + 11) % 12 + 1, time.minute()),
    }
}

/// The minute `minute` of the day (0 to 1439) as a time of day in `hours`.
pub fn time_of_minute(minute: u16, hours: ClockHours) -> String {
    let (hour, rest) = (minute / 60 % 24, minute % 60);
    Time::new(
        i8::try_from(hour).unwrap_or(0),
        i8::try_from(rest).unwrap_or(0),
        0,
        0,
    )
    .map_or_else(|_| String::new(), |time| time_of_day(time, hours))
}

fn day_part(time: Time) -> &'static str {
    match time.hour() < 12 {
        true => "AM",
        false => "PM",
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
    use super::{ClockHours, format_clock, time_digits, time_of_minute, until_next_minute};
    use jiff::civil::{date, time};
    use std::time::Duration;

    #[test]
    fn the_clock_reads_weekday_day_month_and_the_time_in_the_chosen_hours() {
        let cases = [
            (
                date(2026, 9, 24).at(14, 5, 0, 0),
                "Thu 24 Sep 14:05",
                "Thu 24 Sep 2:05 PM",
            ),
            (
                date(2026, 9, 24).at(0, 0, 59, 999_999_999),
                "Thu 24 Sep 00:00",
                "Thu 24 Sep 12:00 AM",
            ),
            (
                date(2026, 1, 1).at(9, 30, 0, 0),
                "Thu 1 Jan 09:30",
                "Thu 1 Jan 9:30 AM",
            ),
            (
                date(2027, 12, 31).at(23, 59, 0, 0),
                "Fri 31 Dec 23:59",
                "Fri 31 Dec 11:59 PM",
            ),
            (
                date(2028, 2, 29).at(12, 0, 0, 0),
                "Tue 29 Feb 12:00",
                "Tue 29 Feb 12:00 PM",
            ),
        ];
        for (now, h24, h12) in cases {
            assert_eq!(format_clock(now, ClockHours::H24), h24, "{now}");
            assert_eq!(format_clock(now, ClockHours::H12), h12, "{now}");
        }
    }

    #[test]
    fn a_minute_of_the_day_reads_as_a_time() {
        let cases = [
            (0, ClockHours::H24, "00:00"),
            (22 * 60, ClockHours::H24, "22:00"),
            (22 * 60, ClockHours::H12, "10:00 PM"),
            (7 * 60 + 5, ClockHours::H12, "7:05 AM"),
        ];
        for (minute, hours, want) in cases {
            assert_eq!(time_of_minute(minute, hours), want, "{minute} {hours:?}");
        }
    }

    #[test]
    fn the_digits_alone_leave_out_am_and_pm() {
        let cases = [
            (time(14, 5, 0, 0), ClockHours::H24, "14:05"),
            (time(14, 5, 0, 0), ClockHours::H12, "2:05"),
            (time(0, 7, 0, 0), ClockHours::H12, "12:07"),
        ];
        for (at, hours, want) in cases {
            assert_eq!(time_digits(at, hours), want, "{at} {hours:?}");
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
