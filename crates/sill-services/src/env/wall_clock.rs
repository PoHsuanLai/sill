//! The wall clock the services read (the bar's time, today's date, a notification's arrival):
//! the system's, or one fixed instant for a test. Timers stay on tokio's time.

use std::fmt;

use jiff::Zoned;
use jiff::tz::TimeZone;

/// The local date and time now.
pub trait WallClock: Send + Sync + fmt::Debug {
    fn now(&self) -> Zoned;
}

/// The system clock in the system time zone, read afresh on every call, so a zone change shows
/// at the next read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SystemClock;

impl WallClock for SystemClock {
    fn now(&self) -> Zoned {
        let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
        Zoned::now().with_time_zone(zone)
    }
}

/// Always this instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedClock(pub Zoned);

impl WallClock for FixedClock {
    fn now(&self) -> Zoned {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use jiff::tz::TimeZone;

    use super::{FixedClock, SystemClock, WallClock};

    #[test]
    fn a_fixed_clock_stands_still_and_the_system_one_moves_on() {
        let at = date(2026, 9, 24)
            .at(14, 5, 0, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap_or_else(|e| panic!("{e}"));
        let fixed = FixedClock(at.clone());
        assert_eq!(fixed.now(), at);
        assert_eq!(fixed.now(), at);
        let before = SystemClock.now();
        assert!(SystemClock.now() >= before);
    }
}
