//! Which month the Calendar widget shows, and the words the grid is titled with: pure, so the
//! view only reads.

use jiff::civil::{Date, Weekday};
use sill_shell_kit::month_grid::data::{MonthKey, MonthStep};

use super::grid::{month_of, shift_month};

/// The month shown, as steps away from the month of the day they were taken on: a turn of the
/// day brings the widget back to today's month.
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
    pub fn month(self, today: Date) -> MonthKey {
        let offset = match self.on == today {
            true => self.offset,
            false => 0,
        };
        let step = if offset >= 0 {
            MonthStep::Next
        } else {
            MonthStep::Previous
        };
        (0..offset.unsigned_abs()).fold(month_of(today), |month, _| shift_month(month, step))
    }

    /// One step from what is shown on `today`.
    pub fn stepped(self, today: Date, step: MonthStep) -> Shown {
        let base = match self.on == today {
            true => self.offset,
            false => 0,
        };
        let offset = match step {
            MonthStep::Next => base.saturating_add(1),
            MonthStep::Previous => base.saturating_sub(1),
        };
        Shown { on: today, offset }
    }
}

/// `September 2026`, or `September` in today's year.
pub(super) fn month_title(month: MonthKey, today: Date) -> String {
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

/// A day's weekday in full, as the Medium card's today column says it ("Sunday").
pub(in crate::widgets) fn weekday_name(day: Date) -> &'static str {
    match day.weekday() {
        Weekday::Monday => "Monday",
        Weekday::Tuesday => "Tuesday",
        Weekday::Wednesday => "Wednesday",
        Weekday::Thursday => "Thursday",
        Weekday::Friday => "Friday",
        Weekday::Saturday => "Saturday",
        Weekday::Sunday => "Sunday",
    }
}

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
    use sill_shell_kit::month_grid::data::{MonthKey, MonthStep};

    use super::{Shown, month_title, weekday_name};
    use crate::widgets::calendar::grid::month_of;

    #[test]
    fn steps_count_from_today_and_reset_when_the_day_turns() {
        let today = date(2026, 12, 20);
        let shown = Shown::today(today)
            .stepped(today, MonthStep::Next)
            .stepped(today, MonthStep::Next);
        assert_eq!(
            shown.month(today),
            MonthKey {
                year: 2027,
                month: 2
            }
        );
        let back = shown.stepped(today, MonthStep::Previous);
        assert_eq!(
            back.month(today),
            MonthKey {
                year: 2027,
                month: 1
            }
        );
        let tomorrow = date(2026, 12, 21);
        assert_eq!(shown.month(tomorrow), month_of(tomorrow));
        assert_eq!(shown.stepped(tomorrow, MonthStep::Previous).offset, -1);
    }

    #[test]
    fn the_today_column_names_the_weekday_in_full() {
        let cases = [
            (date(2026, 9, 27), "Sunday"),
            (date(2026, 9, 28), "Monday"),
            (date(2026, 10, 2), "Friday"),
        ];
        for (day, want) in cases {
            assert_eq!(weekday_name(day), want, "{day}");
        }
    }

    #[test]
    fn titles_name_the_year_only_when_it_is_not_this_one() {
        let today = date(2026, 9, 26);
        let cases = [
            (month_of(today), "September"),
            (
                MonthKey {
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
