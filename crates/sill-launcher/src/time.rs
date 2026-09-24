//! Wall-clock time as an argument (CONVENTIONS §6): frecency ages entries by it, and nothing
//! in this crate reads a clock.

use serde::{Deserialize, Serialize};

/// Seconds since the Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UnixTime(pub u64);

impl UnixTime {
    /// Whole days from `earlier` to `self`; zero when `earlier` is in the future (a clock that
    /// stepped back must not make an entry look older than new).
    pub fn days_since(self, earlier: UnixTime) -> u64 {
        self.0.saturating_sub(earlier.0) / 86_400
    }
}
