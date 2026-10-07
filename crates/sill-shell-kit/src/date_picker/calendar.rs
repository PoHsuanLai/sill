//! The civil calendar a date picker needs, as pure arithmetic on year, month and day: the length
//! of a month, the weekday a date falls on, its ISO week, and a month laid out as the
//! `MonthGrid` draws it. Weeks start on Sunday, as the Mac's default does.

use crate::month_grid::data::{
    DayKey, DayMark, DayPlace, Eventful, IsoWeek, MonthDay, MonthGridData, MonthKey, MonthWeek,
};
use ds::prelude::*;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Whether `year` has a 29th of February.
fn leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// How many days `month` (1 to 12) of `year` has.
pub(crate) fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        2 if leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The days since 1970-01-01 of a civil date (proleptic Gregorian).
fn days_from_civil(year: i32, month: i32, day: i32) -> i32 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let shifted = (month + 9) % 12;
    let day_of_year = (153 * shifted + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The weekday of a civil date, 0 for Sunday to 6 for Saturday.
pub(crate) fn weekday(year: i32, month: i32, day: i32) -> i32 {
    (days_from_civil(year, month, day) + 4).rem_euclid(7)
}

/// The date `days` days after `key`.
fn shifted(key: DayKey, days: i32) -> DayKey {
    let (mut year, mut month, mut day) = (
        i32::from(key.year),
        i32::from(key.month),
        i32::from(key.day) + days,
    );
    while day < 1 {
        month -= 1;
        if month < 1 {
            month = 12;
            year -= 1;
        }
        day += days_in_month(year, month);
    }
    while day > days_in_month(year, month) {
        day -= days_in_month(year, month);
        month += 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
    }
    civil(year, month, day)
}

/// A `DayKey` from a civil date that is in range by construction.
fn civil(year: i32, month: i32, day: i32) -> DayKey {
    DayKey {
        year: i16::try_from(year).unwrap_or(i16::MAX),
        month: i8::try_from(month).unwrap_or(1),
        day: i8::try_from(day).unwrap_or(1),
    }
}

/// The ISO 8601 week of `key`: the week of its Thursday.
fn iso_week(key: DayKey) -> i8 {
    let (year, month, day) = (
        i32::from(key.year),
        i32::from(key.month),
        i32::from(key.day),
    );
    let monday_based = (weekday(year, month, day) + 6) % 7;
    let thursday = shifted(key, 3 - monday_based);
    let start = days_from_civil(i32::from(thursday.year), 1, 1);
    let at = days_from_civil(
        i32::from(thursday.year),
        i32::from(thursday.month),
        i32::from(thursday.day),
    );
    i8::try_from((at - start) / 7 + 1).unwrap_or(1)
}

/// The month `by` months from `month`, years carried.
pub(crate) fn month_after(month: MonthKey, by: i32) -> MonthKey {
    let index = i32::from(month.year) * 12 + i32::from(month.month) - 1 + by;
    MonthKey {
        year: i16::try_from(index.div_euclid(12)).unwrap_or(i16::MAX),
        month: i8::try_from(index.rem_euclid(12) + 1).unwrap_or(1),
    }
}

/// `month` laid out as a grid, `picked` on the accent disc (the grid's mark for the day it
/// stands on), the weeks as many as the month touches.
pub(crate) fn month_grid(month: MonthKey, picked: DayKey) -> MonthGridData {
    let (year, number) = (i32::from(month.year), i32::from(month.month));
    let lead = weekday(year, number, 1);
    let length = days_in_month(year, number);
    let weeks = (lead + length + 6) / 7;
    let first = shifted(civil(year, number, 1), -lead);
    let rows = (0..weeks)
        .map(|week| {
            let days = std::array::from_fn(|at| {
                let key = shifted(first, week * 7 + at as i32);
                let place = match (key.year, key.month).cmp(&(month.year, month.month)) {
                    std::cmp::Ordering::Less => DayPlace::Before,
                    std::cmp::Ordering::Equal => DayPlace::InMonth,
                    std::cmp::Ordering::Greater => DayPlace::After,
                };
                MonthDay {
                    key,
                    place,
                    mark: if key == picked {
                        DayMark::Today
                    } else {
                        DayMark::Plain
                    },
                    events: Eventful::Free,
                }
            });
            MonthWeek {
                number: IsoWeek(iso_week(days[3].key)),
                days,
            }
        })
        .collect();
    MonthGridData {
        month,
        title: TextLine::from(format!(
            "{} {}",
            MONTHS[usize::try_from(number - 1).unwrap_or(0)],
            month.year
        )),
        heads: ["S", "M", "T", "W", "T", "F", "S"].map(TextLine::from),
        weeks: rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{days_in_month, iso_week, month_after, month_grid, weekday};
    use crate::month_grid::data::{DayKey, DayMark, DayPlace, MonthKey};
    use ds::prelude::*;

    fn day(year: i16, month: i8, day: i8) -> DayKey {
        DayKey { year, month, day }
    }

    #[test]
    fn a_month_is_as_long_as_the_calendar_says() {
        const CASES: &[(i32, i32, i32)] = &[
            (2026, 1, 31),
            (2026, 2, 28),
            (2028, 2, 29),
            (1900, 2, 28),
            (2000, 2, 29),
            (2026, 4, 30),
            (2026, 12, 31),
        ];
        for &(year, month, want) in CASES {
            assert_eq!(days_in_month(year, month), want, "{year}-{month}");
        }
    }

    #[test]
    fn a_date_falls_on_its_weekday() {
        // 0 is Sunday.
        const CASES: &[(i32, i32, i32, i32)] = &[
            (1970, 1, 1, 4),
            (2000, 1, 1, 6),
            (2026, 9, 1, 2),
            (2026, 9, 30, 3),
            (2024, 2, 29, 4),
        ];
        for &(year, month, dom, want) in CASES {
            assert_eq!(weekday(year, month, dom), want, "{year}-{month}-{dom}");
        }
    }

    #[test]
    fn the_iso_week_is_the_week_of_the_thursday() {
        const CASES: &[(DayKey, i8)] = &[
            (
                DayKey {
                    year: 2026,
                    month: 1,
                    day: 1,
                },
                1,
            ),
            (
                DayKey {
                    year: 2026,
                    month: 9,
                    day: 30,
                },
                40,
            ),
            (
                DayKey {
                    year: 2021,
                    month: 1,
                    day: 3,
                },
                53,
            ),
            (
                DayKey {
                    year: 2024,
                    month: 12,
                    day: 30,
                },
                1,
            ),
        ];
        for &(key, want) in CASES {
            assert_eq!(iso_week(key), want, "{key:?}");
        }
    }

    #[test]
    fn months_step_across_years() {
        let december = MonthKey {
            year: 2026,
            month: 12,
        };
        assert_eq!(
            month_after(december, 1),
            MonthKey {
                year: 2027,
                month: 1
            }
        );
        let january = MonthKey {
            year: 2026,
            month: 1,
        };
        assert_eq!(
            month_after(january, -1),
            MonthKey {
                year: 2025,
                month: 12
            }
        );
        assert_eq!(month_after(january, 0), january);
    }

    #[test]
    fn september_2026_is_five_rows_led_by_august_and_marks_the_picked_day() {
        let month = MonthKey {
            year: 2026,
            month: 9,
        };
        let grid = month_grid(month, day(2026, 9, 14));
        assert_eq!(grid.weeks.len(), 5);
        let first = grid.weeks[0].days;
        assert_eq!(
            first[0].key,
            day(2026, 8, 30),
            "Sunday the 30th of August leads"
        );
        assert_eq!(first[0].place, DayPlace::Before);
        assert_eq!(first[2].key, day(2026, 9, 1), "the 1st is a Tuesday");
        assert_eq!(first[2].place, DayPlace::InMonth);
        let marked: Vec<DayKey> = grid
            .weeks
            .iter()
            .flat_map(|week| week.days)
            .filter(|cell| cell.mark == DayMark::Today)
            .map(|cell| cell.key)
            .collect();
        assert_eq!(marked, [day(2026, 9, 14)]);
        let last = grid.weeks[4].days;
        assert_eq!(last[6].key, day(2026, 10, 3));
        assert_eq!(last[6].place, DayPlace::After);
        assert_eq!(grid.title, TextLine::from("September 2026"));
    }
}
