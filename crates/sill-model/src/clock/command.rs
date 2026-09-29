//! What the clock service can be told.

/// What the clock service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockCommand {
    /// Re-read the time zone and reformat now (after a resume or a time-zone change).
    Refresh,
}
