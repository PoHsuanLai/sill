//! A month laid out the way the shell lays it out (sill's `month_grid`), so the tests and the
//! gallery have real months to draw: proleptic Gregorian civil dates by day count, the first
//! weekday, the neighbours' padding days, today, the busy days, and each row's ISO week (the
//! week of its Thursday). quire itself never computes a month; this stands in for the caller.

use ds::prelude::*;
use sill_shell_kit::month_grid::data::{
    DayKey, DayMark, DayPlace, Eventful, IsoWeek, MonthDay, MonthGridData, MonthKey, MonthStep,
    MonthWeek,
};

/// Which weekday the rows start on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum First {
    Monday,
    Sunday,
}

const NAMES: [&str; 12] = [
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

/// Days since 1970-01-01 (H. Hinnant's `days_from_civil`).
fn days(key: DayKey) -> i64 {
    let (m, d) = (i64::from(key.month), i64::from(key.day));
    let y = i64::from(key.year) - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date `z` days after 1970-01-01 (`civil_from_days`).
fn civil(z: i64) -> DayKey {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    DayKey {
        year: i16::try_from(year).expect("a sample year fits"),
        month: i8::try_from(month).expect("1-12"),
        day: i8::try_from(day).expect("1-31"),
    }
}

/// 0 for Monday through 6 for Sunday (1970-01-01 was a Thursday).
fn weekday(z: i64) -> i64 {
    (z + 3).rem_euclid(7)
}

/// The ISO week of the day `thursday`: its ordinal in its own year, in sevens.
fn iso_week(thursday: i64) -> IsoWeek {
    let key = civil(thursday);
    let first = days(DayKey {
        month: 1,
        day: 1,
        ..key
    });
    IsoWeek(i8::try_from((thursday - first) / 7 + 1).expect("1-53"))
}

/// The month after or before `month`.
pub fn shift(month: MonthKey, step: MonthStep) -> MonthKey {
    let index = i32::from(month.year) * 12 + i32::from(month.month) - 1;
    let index = match step {
        MonthStep::Previous => index - 1,
        MonthStep::Next => index + 1,
    };
    MonthKey {
        year: i16::try_from(index.div_euclid(12)).expect("a sample year fits"),
        month: i8::try_from(index.rem_euclid(12) + 1).expect("1-12"),
    }
}

/// `month` laid out from `first`, `today` marked, each day in `busy` busy.
pub fn month(month: MonthKey, first: First, today: DayKey, busy: &[DayKey]) -> MonthGridData {
    let start = days(DayKey {
        year: month.year,
        month: month.month,
        day: 1,
    });
    let end = days(shift(month, MonthStep::Next).first()) - 1;
    let lead = match first {
        First::Monday => weekday(start),
        First::Sunday => (weekday(start) + 1) % 7,
    };
    let origin = start - lead;
    let rows = (end - origin) / 7 + 1;
    let thursday = match first {
        First::Monday => 3,
        First::Sunday => 4,
    };
    let weeks = (0..rows)
        .map(|row| {
            let at = origin + row * 7;
            MonthWeek {
                number: iso_week(at + thursday),
                days: std::array::from_fn(|column| {
                    let z = at + i64::try_from(column).expect("0-6");
                    cell(z, start, end, today, busy)
                }),
            }
        })
        .collect();
    MonthGridData {
        month,
        title: TextLine::from(format!(
            "{} {}",
            NAMES[usize::try_from(month.month - 1).expect("1-12")],
            month.year
        )),
        heads: heads(first),
        weeks,
    }
}

fn cell(z: i64, start: i64, end: i64, today: DayKey, busy: &[DayKey]) -> MonthDay {
    let key = civil(z);
    MonthDay {
        key,
        place: match z {
            z if z < start => DayPlace::Before,
            z if z > end => DayPlace::After,
            _ => DayPlace::InMonth,
        },
        mark: if key == today {
            DayMark::Today
        } else {
            DayMark::Plain
        },
        events: if busy.contains(&key) {
            Eventful::Busy
        } else {
            Eventful::Free
        },
    }
}

fn heads(first: First) -> [TextLine; 7] {
    let monday = ["M", "T", "W", "T", "F", "S", "S"];
    std::array::from_fn(|at| {
        let at = match first {
            First::Monday => at,
            First::Sunday => (at + 6) % 7,
        };
        TextLine::from(monday[at])
    })
}

/// Its first day.
trait FirstDay {
    fn first(self) -> DayKey;
}

impl FirstDay for MonthKey {
    fn first(self) -> DayKey {
        DayKey {
            year: self.year,
            month: self.month,
            day: 1,
        }
    }
}

/// September 2026, today the 26th.
pub const SEPTEMBER: MonthKey = MonthKey {
    year: 2026,
    month: 9,
};

/// August 2026: from Monday, six rows (27 July to 6 September), the tallest a month gets.
pub const AUGUST: MonthKey = MonthKey {
    year: 2026,
    month: 8,
};

/// Today in every sample.
pub const TODAY: DayKey = DayKey {
    year: 2026,
    month: 9,
    day: 26,
};

/// The sample's busy days: two in September, today, one on a neighbour's day.
pub const BUSY: [DayKey; 4] = [
    DayKey {
        year: 2026,
        month: 9,
        day: 3,
    },
    DayKey {
        year: 2026,
        month: 9,
        day: 17,
    },
    TODAY,
    DayKey {
        year: 2026,
        month: 10,
        day: 2,
    },
];

/// `month` with the sample's today and busy days.
pub fn sample(key: MonthKey, first: First) -> MonthGridData {
    month(key, first, TODAY, &BUSY)
}
