//! How long to wait before connecting to keycap again.

use std::time::Duration;

/// The wait after the first failed attempt.
const FIRST: Duration = Duration::from_secs(1);

/// The longest wait.
const LONGEST: Duration = Duration::from_secs(30);

/// The wait after one that was `previous` (`None` for the first attempt since the last live
/// connection): one second, doubling to thirty.
pub fn reconnect_delay(previous: Option<Duration>) -> Duration {
    previous.map_or(FIRST, |previous| (previous * 2).min(LONGEST))
}
