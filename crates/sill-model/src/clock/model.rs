//! What the clock service publishes, and how a time of day is written.

use jiff::civil::DateTime;

/// The local date and time of the current minute: what the clock service publishes at each
/// minute boundary. Surfaces write it with [`format_clock`](super::format_clock) in the hours the
/// desktop chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClockMinute(pub DateTime);

/// Whether a time of day is written on a 12-hour or a 24-hour clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ClockHours {
    /// `2:05 PM`.
    H12,
    /// `14:05`.
    #[default]
    H24,
}
