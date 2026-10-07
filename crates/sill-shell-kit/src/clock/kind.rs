//! The words a `ClockFace` is described in (design/04-COMPONENTS.md "Widgets"):
//! the time it shows, whether it is day or night there, and how it is drawn.

use ds::base::word::Word;
use serde::{Deserialize, Serialize};

/// Whether a clock shows its seconds, and which second it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Seconds {
    /// Shown: the second, taken modulo 60. An analog face draws a second hand, a digital one
    /// `hh:mm:ss`.
    Shown(u8),
    /// Hidden: no second hand, `hh:mm`, and a face that need not redraw every second.
    #[default]
    Hidden,
}

impl Seconds {
    /// The second the hands are placed by: a hidden second counts as 0, so the minute hand
    /// stands on the minute.
    pub fn value(self) -> u8 {
        match self {
            Seconds::Shown(second) => second % 60,
            Seconds::Hidden => 0,
        }
    }
}

/// A wall-clock time in some zone. The caller reads the zone; the face only draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ClockTime {
    /// 0 to 23, taken modulo 24.
    pub hour: u8,
    /// 0 to 59, taken modulo 60.
    pub minute: u8,
    /// The second, or none.
    pub second: Seconds,
}

impl ClockTime {
    /// `09:41`, or `09:41:07` with the seconds shown: 24-hour, as the bar clock.
    pub fn digits(self) -> String {
        let (hour, minute) = (self.hour % 24, self.minute % 60);
        match self.second {
            Seconds::Shown(second) => format!("{hour:02}:{minute:02}:{:02}", second % 60),
            Seconds::Hidden => format!("{hour:02}:{minute:02}"),
        }
    }
}

/// Whether it is day or night where the clock is, which tints the analog face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
pub enum DayPhase {
    /// A paper face with ink hands.
    #[default]
    Day,
    /// An ink face with paper hands.
    Night,
}

/// How a clock is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
pub enum ClockLook {
    /// A dial with hands: the medium and large widgets.
    #[default]
    Analog,
    /// The time as digits in the data face: a small widget's row.
    Digital,
}

#[cfg(test)]
mod tests {
    use super::{ClockTime, Seconds};

    #[test]
    fn digits_are_24_hour_and_padded() {
        let cases = [
            (9, 41, Seconds::Hidden, "09:41"),
            (21, 5, Seconds::Shown(7), "21:05:07"),
            (24, 60, Seconds::Shown(61), "00:00:01"),
        ];
        for (hour, minute, second, want) in cases {
            let time = ClockTime {
                hour,
                minute,
                second,
            };
            assert_eq!(time.digits(), want);
        }
    }
}
