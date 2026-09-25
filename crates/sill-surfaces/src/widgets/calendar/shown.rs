//! Which month the Calendar widget shows, and the words it draws: pure, so the view only reads.

use jiff::civil::{Date, Weekday};

use super::grid::{Month, Step, shift_month};

/// The month shown, as steps away from the month of the day they were taken on: a turn of the
/// day brings the widget back to today's month, as the freeze asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Shown {
    /// The day the steps were taken on.
    pub on: Date,
    /// Months after (positive) or before today's month.
    pub offset: i32,
}

impl Shown {
    /// Today's month.
    pub fn today(today: Date) -> Shown {
        Shown {
            on: today,
            offset: 0,
        }
    }

    /// The month shown when today is `today`.
    pub fn month(self, today: Date) -> Month {
        let offset = match self.on == today {
            true => self.offset,
            false => 0,
        };
        let step = match offset >= 0 {
            true => Step::Next,
            false => Step::Previous,
        };
        (0..offset.unsigned_abs()).fold(Month::of(today), |month, _| shift_month(month, step))
    }

    /// One step from what is shown on `today`.
    pub fn stepped(self, today: Date, step: Step) -> Shown {
        let base = match self.on == today {
            true => self.offset,
            false => 0,
        };
        let offset = match step {
            Step::Next => base.saturating_add(1),
            Step::Previous => base.saturating_sub(1),
        };
        Shown { on: today, offset }
    }
}

/// `September 2026`, or `September` in today's year.
pub(super) fn month_title(month: Month, today: Date) -> String {
    let name = MONTHS[usize::try_from(month.month - 1).unwrap_or(0) % 12];
    match month.year == today.year() {
        true => name.to_owned(),
        false => format!("{name} {}", month.year),
    }
}

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

/// A weekday's one-letter head, as macOS's calendar widget draws it.
pub(super) fn initial(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => "M",
        Weekday::Tuesday | Weekday::Thursday => "T",
        Weekday::Wednesday => "W",
        Weekday::Friday => "F",
        Weekday::Saturday | Weekday::Sunday => "S",
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::{Shown, month_title};
    use crate::widgets::calendar::grid::{Month, Step};

    #[test]
    fn steps_count_from_today_and_reset_when_the_day_turns() {
        let today = date(2026, 12, 20);
        let shown = Shown::today(today)
            .stepped(today, Step::Next)
            .stepped(today, Step::Next);
        assert_eq!(
            shown.month(today),
            Month {
                year: 2027,
                month: 2
            }
        );
        let back = shown.stepped(today, Step::Previous);
        assert_eq!(
            back.month(today),
            Month {
                year: 2027,
                month: 1
            }
        );
        let tomorrow = date(2026, 12, 21);
        assert_eq!(shown.month(tomorrow), Month::of(tomorrow));
        assert_eq!(shown.stepped(tomorrow, Step::Previous).offset, -1);
    }

    #[test]
    fn titles_name_the_year_only_when_it_is_not_this_one() {
        let today = date(2026, 9, 26);
        let cases = [
            (Month::of(today), "September"),
            (
                Month {
                    year: 2027,
                    month: 1,
                },
                "January 2027",
            ),
        ];
        for (month, want) in cases {
            assert_eq!(month_title(month, today), want);
        }
    }
}
