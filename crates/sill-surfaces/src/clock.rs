//! The one place a surface turns the wall of `Instant`s into the machines' [`Stamp`]s.

use std::time::Instant;

use sill_services::Stamp;

/// Milliseconds since the daemon's start, as every machine's `now`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameClock {
    origin: Instant,
}

impl FrameClock {
    /// A clock whose zero is `origin` (the daemon passes its start through the root context).
    pub fn new(origin: Instant) -> FrameClock {
        FrameClock { origin }
    }

    /// `at` as a stamp; an instant before the origin is 0.
    pub fn stamp(&self, at: Instant) -> Stamp {
        Stamp(
            u64::try_from(at.saturating_duration_since(self.origin).as_millis())
                .unwrap_or(u64::MAX),
        )
    }

    /// Now, as a stamp.
    pub fn now(&self) -> Stamp {
        self.stamp(Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::FrameClock;
    use sill_services::Stamp;
    use std::time::{Duration, Instant};

    #[test]
    fn stamps_count_milliseconds_from_the_origin() {
        let origin = Instant::now();
        let clock = FrameClock::new(origin);
        assert_eq!(clock.stamp(origin), Stamp(0));
        assert_eq!(
            clock.stamp(origin + Duration::from_millis(1234)),
            Stamp(1234)
        );
        assert_eq!(
            clock.stamp(
                origin
                    .checked_sub(Duration::from_millis(5))
                    .unwrap_or(origin)
            ),
            Stamp(0)
        );
    }
}
