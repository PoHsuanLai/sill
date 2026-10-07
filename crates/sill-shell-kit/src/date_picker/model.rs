//! What a `DatePicker` holds and shows: a date and a time of day, which of them it shows and in
//! which style, and how each segment of the textual style steps (data and pure arithmetic).

use crate::date_picker::calendar::days_in_month;
use crate::month_grid::data::DayKey;
use ds::components::fields::stepper::model::{StepDirection, StepRange};
use ds::prelude::*;

/// How the picker is drawn (`NSDatePicker.Style`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum PickerStyle {
    /// Segments in one line, changed by the arrow keys and a stepper.
    #[default]
    Textual,
    /// A month grid and a time field.
    Graphical,
}

/// What the picker holds a person changing (`NSDatePicker.ElementFlags`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum Elements {
    /// The date only.
    #[default]
    Date,
    /// The time only.
    Time,
    /// Both.
    DateAndTime,
}

/// One segment of the textual style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum Segment {
    /// The month, 1 to 12.
    Month,
    /// The day of the month.
    Day,
    /// The year.
    Year,
    /// The hour, 0 to 23.
    Hour,
    /// The minute, 0 to 59.
    Minute,
}

/// A time of day, on a 24-hour clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TimeOfDay {
    /// 0 to 23.
    pub hour: u8,
    /// 0 to 59.
    pub minute: u8,
}

/// A date and a time: what a picker holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DateValue {
    /// The day.
    pub day: DayKey,
    /// The time of day.
    pub time: TimeOfDay,
}

impl DateValue {
    /// The values `segment` can take here: a day's range is the length of its month.
    pub fn range(self, segment: Segment) -> StepRange {
        match segment {
            Segment::Month => StepRange::new(1, 12, 1),
            Segment::Day => StepRange::new(
                1,
                days_in_month(i32::from(self.day.year), i32::from(self.day.month)),
                1,
            ),
            Segment::Year => StepRange::new(1, 9999, 1),
            Segment::Hour => StepRange::new(0, 23, 1),
            Segment::Minute => StepRange::new(0, 59, 1),
        }
    }

    /// What `segment` reads.
    pub fn value(self, segment: Segment) -> i32 {
        match segment {
            Segment::Month => i32::from(self.day.month),
            Segment::Day => i32::from(self.day.day),
            Segment::Year => i32::from(self.day.year),
            Segment::Hour => i32::from(self.time.hour),
            Segment::Minute => i32::from(self.time.minute),
        }
    }

    /// The same moment with `segment` set to `to`, held in its range; a month or year that leaves
    /// the day past the end of its month brings the day back to that month's last.
    pub fn with(self, segment: Segment, to: i32) -> DateValue {
        let to = self.range(segment).clamp(to);
        let (year, month, day) = (
            i32::from(self.day.year),
            i32::from(self.day.month),
            i32::from(self.day.day),
        );
        let (year, month, day) = match segment {
            Segment::Year => (to, month, day),
            Segment::Month => (year, to, day),
            Segment::Day => (year, month, to),
            Segment::Hour | Segment::Minute => (year, month, day),
        };
        let day = day.min(days_in_month(year, month));
        DateValue {
            day: DayKey {
                year: i16::try_from(year).unwrap_or(i16::MAX),
                month: i8::try_from(month).unwrap_or(1),
                day: i8::try_from(day).unwrap_or(1),
            },
            time: TimeOfDay {
                hour: if segment == Segment::Hour {
                    u8::try_from(to).unwrap_or(0)
                } else {
                    self.time.hour
                },
                minute: if segment == Segment::Minute {
                    u8::try_from(to).unwrap_or(0)
                } else {
                    self.time.minute
                },
            },
        }
    }

    /// One step of `segment`: the month, day, hour and minute wrap round at their ends (December
    /// steps to January), the year stops at its own.
    pub fn stepped(self, segment: Segment, direction: StepDirection) -> DateValue {
        let range = self.range(segment);
        let now = self.value(segment);
        let next = range.step_from(now, direction);
        let wraps = !matches!(segment, Segment::Year);
        let to = match (wraps, next == range.clamp(now)) {
            (true, true) => match direction {
                StepDirection::Up => range.min(),
                StepDirection::Down => range.max(),
            },
            _ => next,
        };
        self.with(segment, to)
    }
}

#[cfg(test)]
mod tests {
    use super::{DateValue, Segment, TimeOfDay};
    use crate::month_grid::data::DayKey;
    use ds::components::fields::stepper::model::StepDirection;

    fn value(year: i16, month: i8, day: i8, hour: u8, minute: u8) -> DateValue {
        DateValue {
            day: DayKey { year, month, day },
            time: TimeOfDay { hour, minute },
        }
    }

    #[test]
    fn a_segment_steps_and_wraps_at_its_ends() {
        /// name, segment, direction, and the year, month, day, hour, minute wanted.
        type Case = (&'static str, Segment, StepDirection, (i16, i8, i8, u8, u8));
        const CASES: &[Case] = &[
            (
                "day up",
                Segment::Day,
                StepDirection::Up,
                (2026, 9, 15, 9, 5),
            ),
            (
                "day wraps at the end of the month",
                Segment::Day,
                StepDirection::Up,
                (2026, 9, 1, 9, 5),
            ),
            (
                "month up",
                Segment::Month,
                StepDirection::Up,
                (2026, 10, 14, 9, 5),
            ),
            (
                "december wraps to january",
                Segment::Month,
                StepDirection::Up,
                (2026, 1, 14, 9, 5),
            ),
            (
                "year down",
                Segment::Year,
                StepDirection::Down,
                (2025, 9, 14, 9, 5),
            ),
            (
                "hour wraps down",
                Segment::Hour,
                StepDirection::Down,
                (2026, 9, 14, 23, 5),
            ),
            (
                "minute up",
                Segment::Minute,
                StepDirection::Up,
                (2026, 9, 14, 9, 6),
            ),
        ];
        let starts = [
            value(2026, 9, 14, 9, 5),
            value(2026, 9, 30, 9, 5),
            value(2026, 9, 14, 9, 5),
            value(2026, 12, 14, 9, 5),
            value(2026, 9, 14, 9, 5),
            value(2026, 9, 14, 0, 5),
            value(2026, 9, 14, 9, 5),
        ];
        for (&(name, segment, direction, want), start) in CASES.iter().zip(starts) {
            let (year, month, day, hour, minute) = want;
            assert_eq!(
                start.stepped(segment, direction),
                value(year, month, day, hour, minute),
                "{name}"
            );
        }
    }

    #[test]
    fn a_month_that_is_shorter_brings_the_day_back_into_it() {
        let stepped = value(2026, 1, 31, 0, 0).stepped(Segment::Month, StepDirection::Up);
        assert_eq!(stepped, value(2026, 2, 28, 0, 0));
        let leap = value(2028, 2, 29, 0, 0).with(Segment::Year, 2027);
        assert_eq!(leap, value(2027, 2, 28, 0, 0));
    }

    #[test]
    fn a_typed_value_is_held_in_the_segments_range() {
        let start = value(2026, 9, 14, 9, 5);
        assert_eq!(start.with(Segment::Hour, 40), value(2026, 9, 14, 23, 5));
        assert_eq!(start.with(Segment::Day, 31), value(2026, 9, 30, 9, 5));
    }
}
