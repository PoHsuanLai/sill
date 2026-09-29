//! The month grid, computed: pure over the month, the first weekday, today and the busy days,
//! so the widget and quire's `MonthGrid` take the same data.

use jiff::ToSpan;
use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use sill_services::{CalEvent, EventTime};
use sill_settings::FirstWeekday;

/// A calendar month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Month {
    pub year: i16,
    /// 1-12.
    pub month: i8,
}

impl Month {
    /// The month `date` is in.
    pub fn of(date: Date) -> Month {
        Month {
            year: date.year(),
            month: date.month(),
        }
    }
}

/// Previous or next month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    Previous,
    Next,
}

/// An ISO 8601 week number, 1-53.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IsoWeek(pub i8);

/// Whether a cell's day is in the month shown or pads the first or last week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DayPlace {
    Before,
    InMonth,
    After,
}

/// Whether a cell is today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DayMark {
    Today,
    Plain,
}

/// Whether a day has an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Eventful {
    Busy,
    Free,
}

/// One cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridDay {
    pub date: Date,
    pub place: DayPlace,
    pub mark: DayMark,
    pub events: Eventful,
}

/// One row: its ISO week (the week of its Thursday, whatever the first weekday) and its seven
/// days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridWeek {
    pub number: IsoWeek,
    pub days: [GridDay; 7],
}

/// A month laid out in weeks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MonthGrid {
    pub month: Month,
    /// The column heads, starting at the first weekday.
    pub weekdays: [Weekday; 7],
    /// As many rows as the month touches (four to six), padded with the neighbours' days.
    pub weeks: Vec<GridWeek>,
}

/// The month after or before `month`.
pub fn shift_month(month: Month, step: Step) -> Month {
    match (step, month.month) {
        (Step::Next, 12) => Month {
            year: month.year.saturating_add(1),
            month: 1,
        },
        (Step::Next, m) => Month {
            year: month.year,
            month: m + 1,
        },
        (Step::Previous, 1) => Month {
            year: month.year.saturating_sub(1),
            month: 12,
        },
        (Step::Previous, m) => Month {
            year: month.year,
            month: m - 1,
        },
    }
}

/// `month` laid out from `first`, `today` marked, each day in `busy` marked busy.
pub fn month_grid(month: Month, first: FirstWeekday, today: Date, busy: &[Date]) -> MonthGrid {
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
    MonthGrid {
        month,
        weekdays,
        weeks,
    }
}

/// The month's first day (a month out of range reads as January 1970's, never a panic).
fn first_of(month: Month) -> Date {
    Date::new(month.year, month.month, 1).unwrap_or(Date::constant(1970, 1, 1))
}

/// One row from `start`.
fn week_of(start: Date, month: Month, today: Date, busy: &[Date]) -> GridWeek {
    let days: [GridDay; 7] = std::array::from_fn(|i| {
        let date = start.saturating_add((i as i64).days());
        GridDay {
            date,
            place: match Month::of(date).cmp(&month) {
                std::cmp::Ordering::Less => DayPlace::Before,
                std::cmp::Ordering::Equal => DayPlace::InMonth,
                std::cmp::Ordering::Greater => DayPlace::After,
            },
            mark: match date == today {
                true => DayMark::Today,
                false => DayMark::Plain,
            },
            events: match busy.contains(&date) {
                true => Eventful::Busy,
                false => Eventful::Free,
            },
        }
    });
    let thursday = days
        .iter()
        .find(|day| day.date.weekday() == Weekday::Thursday)
        .map_or(start, |day| day.date);
    GridWeek {
        number: IsoWeek(thursday.iso_week_date().week()),
        days,
    }
}

/// The days of `month` that have at least one event in `zone` (an all-day event marks each of
/// its days, a timed one each local day it touches), sorted, once each.
pub fn busy_days(events: &[CalEvent], zone: &TimeZone, month: Month) -> Vec<Date> {
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
    use jiff::civil::{Weekday, date};
    use sill_settings::FirstWeekday;

    use super::{DayMark, DayPlace, Eventful, IsoWeek, Month, Step, month_grid, shift_month};

    const SEPT: Month = Month {
        year: 2026,
        month: 9,
    };

    #[test]
    fn a_month_is_the_one_a_date_is_in() {
        assert_eq!(Month::of(date(2026, 9, 26)), SEPT);
    }

    #[test]
    fn months_step_across_years() {
        let cases = [
            (
                SEPT,
                Step::Next,
                Month {
                    year: 2026,
                    month: 10,
                },
            ),
            (
                Month {
                    year: 2026,
                    month: 12,
                },
                Step::Next,
                Month {
                    year: 2027,
                    month: 1,
                },
            ),
            (
                Month {
                    year: 2026,
                    month: 1,
                },
                Step::Previous,
                Month {
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
        assert_eq!(monday.weekdays[0], Weekday::Monday);
        assert_eq!(monday.weeks.len(), 5);
        let first = monday.weeks[0];
        assert_eq!(first.number, IsoWeek(36));
        assert_eq!(first.days[0].date, date(2026, 8, 31));
        assert_eq!(first.days[0].place, DayPlace::Before);
        assert_eq!(first.days[1].date, date(2026, 9, 1));
        assert_eq!(first.days[1].place, DayPlace::InMonth);
        let week = monday.weeks[3];
        assert_eq!(week.days[5].date, today);
        assert_eq!(week.days[5].mark, DayMark::Today);
        assert_eq!(monday.weeks[4].days[0].events, Eventful::Busy);
        assert_eq!(monday.weeks[4].days[3].place, DayPlace::After);

        let sunday = month_grid(SEPT, FirstWeekday::Sunday, today, &[]);
        assert_eq!(sunday.weekdays[0], Weekday::Sunday);
        assert_eq!(sunday.weeks[0].days[0].date, date(2026, 8, 30));
        assert_eq!(
            sunday.weeks[0].number,
            IsoWeek(36),
            "the week of its Thursday"
        );
        assert_eq!(sunday.weeks.len(), 5);
    }

    #[test]
    fn a_february_that_fits_four_weeks_has_four() {
        let feb = Month {
            year: 2027,
            month: 2,
        };
        let grid = month_grid(feb, FirstWeekday::Monday, date(2027, 2, 1), &[]);
        assert_eq!(grid.weeks.len(), 4);
    }
}
