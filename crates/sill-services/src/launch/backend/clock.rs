//! The monotonic clock the timing lines are read from.

use rustix::time::{ClockId, clock_gettime};
use sill_model::launch::Stamp;

/// Now on `CLOCK_MONOTONIC`: the clock the casement and workflow scripts read too.
pub fn monotonic_now() -> Stamp {
    let reading = clock_gettime(ClockId::Monotonic);
    // A monotonic reading is never negative; a bad one would read as zero rather than wrap.
    let seconds = u64::try_from(reading.tv_sec).unwrap_or(0);
    let nanos = u64::try_from(reading.tv_nsec).unwrap_or(0);
    Stamp::of_reading(seconds, nanos)
}

#[cfg(test)]
mod tests {
    use super::monotonic_now;

    #[test]
    fn the_clock_does_not_run_backwards_and_has_run_for_a_while() {
        let (first, second) = (monotonic_now(), monotonic_now());
        assert!(second >= first);
        assert!(first.0 > 0, "the machine has been up for some time");
    }
}
