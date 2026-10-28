//! The month grid, computed: pure over the month, the first weekday, today and the busy days,
//! so the widget and quire's `MonthGrid` take the same data.

use std::cmp::Ordering;

use jiff::ToSpan;
use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use sill_model::events::{CalEvent, EventTime};
use sill_settings::FirstWeekday;
use sill_shell_kit::month_grid::data::{
    DayKey, DayMark, DayPlace, Eventful, IsoWeek, MonthDay, MonthGridData, MonthKey, MonthStep,
    MonthWeek,
};

use super::shown::{initial, month_title};

/// The month `date` is in.
pub(super) fn month_of(date: Date) -> MonthKey {
    MonthKey {
        year: date.year(),
        month: date.month(),
    }
}

/// The month after or before `month`.
pub fn shift_month(month: MonthKey, step: MonthStep) -> MonthKey {
    match (step, month.month) {
        (MonthStep::Next, 12) => MonthKey {
            year: month.year.saturating_add(1),
            month: 1,
        },
        (MonthStep::Next, m) => MonthKey {
            year: month.year,
            month: m + 1,
        },
        (MonthStep::Previous, 1) => MonthKey {
            year: month.year.saturating_sub(1),
            month: 12,
        },
        (MonthStep::Previous, m) => MonthKey {
            year: month.year,
            month: m - 1,
        },
    }
}

/// `month` laid out from `first`, `today` marked, each day in `busy` marked busy, with its title
/// and weekday initials in words.
pub fn month_grid(
    month: MonthKey,
    first: FirstWeekday,
    today: Date,
    busy: &[Date],
) -> MonthGridData {
    let head = match first {
        FirstWeekday::Monday => Weekday::Monday,
        FirstWeekday::Sunday => Weekday::Sunday,
    };
    let weekdays: [Weekday; 7] = std::array::from_fn(|i| head.wrapping_add(i as i64));
    let opening = first_of(month);
    let back = i64::from(opening.weekday().since(head));
    let start = opening.saturating_sub(back.days());
    let closing = opening.last_of_month();
    let weeks = (0..6)
        .map(|row| start.saturating_add((row * 7).days()))
        .take_while(|row_start| *row_start <= closing)
        .map(|row_start| week_of(row_start, month, today, busy))
        .collect();
    MonthGridData {
        month,
        title: month_title(month, today).into(),
        heads: weekdays.map(|day| initial(day).into()),
        weeks,
    }
}

/// The month's first day (a month out of range reads as January 1970's, never a panic).
fn first_of(month: MonthKey) -> Date {
    Date::new(month.year, month.month, 1).unwrap_or(Date::constant(1970, 1, 1))
}

/// One row from `start`: its ISO week is the week of its Thursday, whatever the first weekday.
fn week_of(start: Date, month: MonthKey, today: Date, busy: &[Date]) -> MonthWeek {
    let dates: [Date; 7] = std::array::from_fn(|i| start.saturating_add((i as i64).days()));
    let thursday = dates
        .into_iter()
        .find(|date| date.weekday() == Weekday::Thursday)
        .unwrap_or(start);
    MonthWeek {
        number: IsoWeek(thursday.iso_week_date().week()),
        days: dates.map(|date| day_of(date, month, today, busy)),
    }
}

/// One cell.
fn day_of(date: Date, month: MonthKey, today: Date, busy: &[Date]) -> MonthDay {
    let place = match month_of(date).cmp(&month) {
        Ordering::Less => DayPlace::Before,
        Ordering::Equal => DayPlace::InMonth,
        Ordering::Greater => DayPlace::After,
    };
    let mark = if date == today {
        DayMark::Today
    } else {
        DayMark::Plain
    };
    let events = if busy.contains(&date) {
        Eventful::Busy
    } else {
        Eventful::Free
    };
    MonthDay {
        key: DayKey {
            year: date.year(),
            month: date.month(),
            day: date.day(),
        },
        place,
        mark,
        events,
    }
}

/// The days of `month` that have at least one event in `zone` (an all-day event marks each of
/// its days, a timed one each local day it touches), sorted, once each.
pub fn busy_days(events: &[CalEvent], zone: &TimeZone, month: MonthKey) -> Vec<Date> {
    let opening = first_of(month);
    let closing = opening.last_of_month();
    let mut days: Vec<Date> = events
        .iter()
        .filter_map(|event| local_days(event, zone))
        .flat_map(|(first, last)| {
            let from = first.max(opening);
            let to = last.min(closing);
            let count = from.until(to).map_or(-1, |span| i64::from(span.get_days()));
            (0..=count).map(move |i| from.saturating_add(i.days()))
        })
        .collect();
    days.sort_unstable();
    days.dedup();
    days
}

/// The local days `event` touches, first and last: a timed event ending at midnight does not
/// touch the day it ends on, a moment touches its own.
fn local_days(event: &CalEvent, zone: &TimeZone) -> Option<(Date, Date)> {
    match event.when {
        EventTime::AllDay { first, last } => Some((first, last)),
        EventTime::Timed { start, end } => {
            let first = start.to_zoned(zone.clone()).date();
            let last = match end > start {
                true => end
                    .checked_sub(1.nanosecond())
                    .ok()?
                    .to_zoned(zone.clone())
                    .date(),
                false => first,
            };
            Some((first, last.max(first)))
        }
    }
}

#[cfg(test)]
mod tests {
    use ds::prelude::*;
    use jiff::civil::date;
    use sill_settings::FirstWeekday;
    use sill_shell_kit::month_grid::data::{
        DayKey, DayMark, DayPlace, Eventful, IsoWeek, MonthKey, MonthStep,
    };

    use super::{month_grid, month_of, shift_month};

    const SEPT: MonthKey = MonthKey {
        year: 2026,
        month: 9,
    };

    fn key(year: i16, month: i8, day: i8) -> DayKey {
        DayKey { year, month, day }
    }

    #[test]
    fn a_month_is_the_one_a_date_is_in() {
        assert_eq!(month_of(date(2026, 9, 26)), SEPT);
    }

    #[test]
    fn months_step_across_years() {
        let cases = [
            (
                SEPT,
                MonthStep::Next,
                MonthKey {
                    year: 2026,
                    month: 10,
                },
            ),
            (
                MonthKey {
                    year: 2026,
                    month: 12,
                },
                MonthStep::Next,
                MonthKey {
                    year: 2027,
                    month: 1,
                },
            ),
            (
                MonthKey {
                    year: 2026,
                    month: 1,
                },
                MonthStep::Previous,
                MonthKey {
                    year: 2025,
                    month: 12,
                },
            ),
        ];
        for (month, step, want) in cases {
            assert_eq!(shift_month(month, step), want, "{month:?} {step:?}");
        }
    }

    #[test]
    fn september_2026_from_monday_and_from_sunday() {
        let today = date(2026, 9, 26);
        let busy = [date(2026, 9, 28)];
        let monday = month_grid(SEPT, FirstWeekday::Monday, today, &busy);
        let heads: Vec<String> = monday.heads.iter().map(TextLine::plain_text).collect();
        assert_eq!(heads, ["M", "T", "W", "T", "F", "S", "S"]);
        assert_eq!(monday.month, SEPT);
        assert_eq!(monday.title.plain_text(), "September");
        assert_eq!(monday.weeks.len(), 5);
        let first = monday.weeks[0];
        assert_eq!(first.number, IsoWeek(36));
        assert_eq!(first.days[0].key, key(2026, 8, 31));
        assert_eq!(first.days[0].place, DayPlace::Before);
        assert_eq!(first.days[1].key, key(2026, 9, 1));
        assert_eq!(first.days[1].place, DayPlace::InMonth);
        let week = monday.weeks[3];
        assert_eq!(week.days[5].key, key(2026, 9, 26));
        assert_eq!(week.days[5].mark, DayMark::Today);
        assert_eq!(week.days[4].mark, DayMark::Plain);
        assert_eq!(monday.weeks[4].days[0].events, Eventful::Busy);
        assert_eq!(monday.weeks[4].days[1].events, Eventful::Free);
        assert_eq!(monday.weeks[4].days[3].place, DayPlace::After);

        let sunday = month_grid(SEPT, FirstWeekday::Sunday, today, &[]);
        let heads: Vec<String> = sunday.heads.iter().map(TextLine::plain_text).collect();
        assert_eq!(heads, ["S", "M", "T", "W", "T", "F", "S"]);
        assert_eq!(sunday.weeks[0].days[0].key, key(2026, 8, 30));
        assert_eq!(
            sunday.weeks[0].number,
            IsoWeek(36),
            "the week of its Thursday"
        );
        assert_eq!(sunday.weeks.len(), 5);
    }

    #[test]
    fn a_february_that_fits_four_weeks_has_four() {
        let feb = MonthKey {
            year: 2027,
            month: 2,
        };
        let grid = month_grid(feb, FirstWeekday::Monday, date(2027, 2, 1), &[]);
        assert_eq!(grid.weeks.len(), 4);
    }
}
