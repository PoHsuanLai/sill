//! Which month the Calendar widget shows, and the grid as quire's `MonthGrid` takes it: pure, so
//! the view only reads.

use jiff::civil::{Date, Weekday};

use super::grid::{
    DayMark, DayPlace, Eventful, GridDay, GridWeek, Month, MonthGrid, Step, shift_month,
};

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

/// `grid` as quire's `MonthGrid` draws it, field for field, with the title and the weekday
/// initials in words.
pub(super) fn grid_data(grid: &MonthGrid, today: Date) -> ds::MonthGridData {
    ds::MonthGridData {
        month: ds::MonthKey {
            year: grid.month.year,
            month: grid.month.month,
        },
        title: month_title(grid.month, today).into(),
        heads: grid.weekdays.map(|day| initial(day).into()),
        weeks: grid.weeks.iter().map(week_data).collect(),
    }
}

fn week_data(week: &GridWeek) -> ds::MonthWeek {
    ds::MonthWeek {
        number: ds::IsoWeek(week.number.0),
        days: week.days.map(|day| day_data(&day)),
    }
}

fn day_data(day: &GridDay) -> ds::MonthDay {
    ds::MonthDay {
        key: ds::DayKey {
            year: day.date.year(),
            month: day.date.month(),
            day: day.date.day(),
        },
        place: match day.place {
            DayPlace::Before => ds::DayPlace::Before,
            DayPlace::InMonth => ds::DayPlace::InMonth,
            DayPlace::After => ds::DayPlace::After,
        },
        mark: match day.mark {
            DayMark::Today => ds::DayMark::Today,
            DayMark::Plain => ds::DayMark::Plain,
        },
        events: match day.events {
            Eventful::Busy => ds::Eventful::Busy,
            Eventful::Free => ds::Eventful::Free,
        },
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

    use sill_settings::FirstWeekday;

    use super::{Shown, grid_data, month_title};
    use crate::widgets::calendar::grid::month_grid;
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
    fn the_grid_maps_to_quires_field_for_field() {
        let today = date(2026, 9, 26);
        let busy = [date(2026, 9, 28)];
        let grid = month_grid(Month::of(today), FirstWeekday::Sunday, today, &busy);
        let data = grid_data(&grid, today);
        assert_eq!(
            data.month,
            ds::MonthKey {
                year: 2026,
                month: 9
            }
        );
        assert_eq!(data.title.plain_text(), "September");
        let heads: Vec<String> = data.heads.iter().map(ds::Text::plain_text).collect();
        assert_eq!(heads, ["S", "M", "T", "W", "T", "F", "S"]);
        assert_eq!(data.weeks.len(), grid.weeks.len());
        let days: Vec<&ds::MonthDay> = data.weeks.iter().flat_map(|w| w.days.iter()).collect();
        let marked: Vec<_> = days
            .iter()
            .filter(|d| d.mark == ds::DayMark::Today)
            .map(|d| d.key)
            .collect();
        assert_eq!(
            marked,
            [ds::DayKey {
                year: 2026,
                month: 9,
                day: 26
            }]
        );
        let busy_days: Vec<_> = days
            .iter()
            .filter(|d| d.events == ds::Eventful::Busy)
            .map(|d| d.key.day)
            .collect();
        assert_eq!(busy_days, [28]);
        assert_eq!(days[0].place, ds::DayPlace::Before, "30 August");
        assert_eq!(data.weeks[0].number, ds::IsoWeek(36));
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
