//! The wall clock the providers and the runner age their rows against.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use sill_launcher::UnixTime;

/// The wall clock, in whole seconds since the epoch; a clock that cannot be read (never
/// observed, but not a reason to panic a keystroke) ages every entry from year 1970.
pub(crate) fn unix_now() -> UnixTime {
    UnixTime(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs()),
    )
}

/// `time` in whole seconds since the epoch, or `None` for one before it.
pub(crate) fn unix_seconds(time: SystemTime) -> Option<UnixTime> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|since| UnixTime(since.as_secs()))
}

/// The modification time of the file at `path`, or `None` when it cannot be read.
pub(crate) fn modified_of(path: &Path) -> Option<UnixTime> {
    unix_seconds(std::fs::metadata(path).ok()?.modified().ok()?)
}
