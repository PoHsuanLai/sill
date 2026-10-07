//! A timeline as another process sends it (design/23-WIDGETS.md section 9.5). An instant cannot
//! cross a process (each has its own monotonic clock, and a test's is virtual), so on the wire
//! each entry's date is an offset from when the timeline was sent, in milliseconds, and the
//! receiver dates it from when it arrived. The transport (a D-Bus signal carrying this as JSON)
//! is not built yet; this is the format it will carry, so an app can write it today.

use crate::widget::timeline::{Dated, EntryDate, Refresh, Timeline};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// An entry on the wire: shown `after_ms` after the timeline was sent (0: at once).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WireEntry<E> {
    /// Milliseconds after sending.
    pub after_ms: u64,
    /// The entry.
    pub entry: E,
}

/// The refresh policy on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireRefresh {
    /// [`Refresh::Never`].
    Never,
    /// [`Refresh::AtEnd`].
    AtEnd,
    /// [`Refresh::After`], this many milliseconds after sending.
    AfterMs(u64),
}

/// A timeline on the wire: `{"entries":[{"after_ms":0,"entry":…}],"refresh":"never"}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WireTimeline<E> {
    /// The entries, any order.
    pub entries: Vec<WireEntry<E>>,
    /// The refresh policy.
    pub refresh: WireRefresh,
}

/// Milliseconds as a duration.
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// Whole milliseconds from `sent` to `at` (0 if earlier).
fn ms_since(at: Instant, sent: Instant) -> u64 {
    u64::try_from(at.saturating_duration_since(sent).as_millis()).unwrap_or(u64::MAX)
}

impl<E> WireTimeline<E> {
    /// The timeline dated from `arrived`, the receiver's now when it came in.
    pub fn received(self, arrived: Instant) -> Timeline<E> {
        let entries = self
            .entries
            .into_iter()
            .map(|wire| {
                let date = match wire.after_ms {
                    0 => EntryDate::Start,
                    after => EntryDate::At(arrived + ms(after)),
                };
                Dated::new(date, wire.entry)
            })
            .collect();
        let refresh = match self.refresh {
            WireRefresh::Never => Refresh::Never,
            WireRefresh::AtEnd => Refresh::AtEnd,
            WireRefresh::AfterMs(after) => Refresh::After(arrived + ms(after)),
        };
        Timeline::new(entries, refresh)
    }

    /// `timeline` as a sender at `sent` writes it.
    pub fn sent(timeline: Timeline<E>, sent: Instant) -> Self
    where
        E: Clone,
    {
        let entries = timeline
            .entries()
            .iter()
            .map(|dated| WireEntry {
                after_ms: match dated.date {
                    EntryDate::Start => 0,
                    EntryDate::At(at) => ms_since(at, sent),
                },
                entry: dated.entry.clone(),
            })
            .collect();
        let refresh = match timeline.refresh() {
            Refresh::Never => WireRefresh::Never,
            Refresh::AtEnd => WireRefresh::AtEnd,
            Refresh::After(at) => WireRefresh::AfterMs(ms_since(at, sent)),
        };
        WireTimeline { entries, refresh }
    }
}
