//! Where a clock's hands point, as pure arithmetic (design/04-COMPONENTS.md "Widgets"): kept
//! apart from the drawing so a table can pin every hand without rendering a face.
//!
//! Angles are whole tenths of a degree clockwise from twelve, so the table compares integers
//! and the drawing writes `302.5` rather than a float's rounding.

use crate::clock::kind::ClockTime;

/// An angle clockwise from twelve, in tenths of a degree: 0 to 3599.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Tenths(pub u16);

impl Tenths {
    /// The angle as an SVG `rotate()` number: `302.5`, `90`.
    pub fn css(self) -> String {
        match self.0 % 10 {
            0 => format!("{}", self.0 / 10),
            tenth => format!("{}.{tenth}", self.0 / 10),
        }
    }
}

/// The three hands' angles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Hands {
    /// The hour hand: 30 degrees an hour, half a degree a minute, a 120th a second.
    pub hour: Tenths,
    /// The minute hand: 6 degrees a minute, a tenth a second.
    pub minute: Tenths,
    /// The second hand: 6 degrees a second.
    pub second: Tenths,
}

/// Where the hands stand at `time`. A hidden second counts as 0.
pub fn hands(time: ClockTime) -> Hands {
    let hour = u16::from(time.hour % 12);
    let minute = u16::from(time.minute % 60);
    let second = u16::from(time.second.value());
    Hands {
        hour: Tenths(hour * 300 + minute * 5 + second / 12),
        minute: Tenths(minute * 60 + second),
        second: Tenths(second * 60),
    }
}

#[cfg(test)]
mod tests {
    use super::{Hands, Tenths, hands};
    use crate::clock::kind::{ClockTime, Seconds};

    fn at(hour: u8, minute: u8, second: Seconds) -> ClockTime {
        ClockTime {
            hour,
            minute,
            second,
        }
    }

    #[test]
    fn the_hands_stand_where_a_clock_would() {
        let cases = [
            (at(0, 0, Seconds::Hidden), (0, 0, 0)),
            (at(12, 0, Seconds::Hidden), (0, 0, 0)),
            (at(3, 0, Seconds::Hidden), (900, 0, 0)),
            (at(15, 0, Seconds::Hidden), (900, 0, 0)),
            (at(6, 30, Seconds::Hidden), (1950, 1800, 0)),
            (at(9, 45, Seconds::Hidden), (2925, 2700, 0)),
            (at(10, 10, Seconds::Shown(30)), (3052, 630, 1800)),
            (at(23, 59, Seconds::Shown(59)), (3599, 3599, 3540)),
            (at(1, 0, Seconds::Shown(12)), (301, 12, 720)),
            (at(25, 61, Seconds::Shown(60)), (305, 60, 0)),
        ];
        for (time, (hour, minute, second)) in cases {
            let want = Hands {
                hour: Tenths(hour),
                minute: Tenths(minute),
                second: Tenths(second),
            };
            assert_eq!(hands(time), want, "{time:?}");
        }
    }

    #[test]
    fn angles_are_written_without_trailing_zeros() {
        let cases = [(0, "0"), (900, "90"), (3025, "302.5"), (3599, "359.9")];
        for (tenths, want) in cases {
            assert_eq!(Tenths(tenths).css(), want);
        }
    }
}
