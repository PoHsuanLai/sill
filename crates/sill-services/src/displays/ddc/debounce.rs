//! Write coalescing: a slider dragged across a monitor's brightness produces dozens of values a
//! second, and a DDC write takes ~50 ms plus the monitor's own settling time. Only the latest
//! value per (display, feature) is kept, and it goes out once nothing newer has arrived for
//! [`QUIET`]. Pure: time is a [`Stamp`] argument.

use super::bus::DdcId;
use super::vcp::Feature;
use crate::units::Stamp;

/// How long a value must stand before it is written.
pub const QUIET: u64 = 120;

/// One write waiting for its turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Write {
    pub id: DdcId,
    pub feature: Feature,
    pub value: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pending {
    write: Write,
    due: Stamp,
}

/// Writes not yet due.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Debounce {
    pending: Vec<Pending>,
}

impl Debounce {
    /// `self` with `write` replacing any pending write of the same feature on the same display,
    /// due [`QUIET`] ms after `now`.
    pub fn push(self, now: Stamp, write: Write) -> Debounce {
        let pending = self
            .pending
            .into_iter()
            .filter(|p| (p.write.id, p.write.feature) != (write.id, write.feature))
            .chain(std::iter::once(Pending {
                write,
                due: now.after(QUIET),
            }))
            .collect();
        Debounce { pending }
    }

    /// The writes due at `now`, in the order they were pushed, and what is left.
    pub fn take_due(self, now: Stamp) -> (Debounce, Vec<Write>) {
        let (due, waiting): (Vec<Pending>, Vec<Pending>) =
            self.pending.into_iter().partition(|p| p.due <= now);
        (
            Debounce { pending: waiting },
            due.into_iter().map(|p| p.write).collect(),
        )
    }

    /// When the next write falls due, if any is waiting.
    pub fn next_due(&self) -> Option<Stamp> {
        self.pending.iter().map(|p| p.due).min()
    }
}

#[cfg(test)]
mod tests {
    use super::{Debounce, Write};
    use crate::displays::ddc::{DdcId, Feature};
    use crate::units::Stamp;

    fn brightness(value: u16) -> Write {
        Write {
            id: DdcId(5),
            feature: Feature::Brightness,
            value,
        }
    }

    #[test]
    fn the_latest_value_goes_out_after_a_quiet_spell() {
        let volume = Write {
            feature: Feature::SpeakerVolume,
            ..brightness(20)
        };
        let other_display = Write {
            id: DdcId(6),
            ..brightness(70)
        };
        // A drag: 40 at 0 ms, 45 at 50, 50 at 100; a volume change at 60; another display at 90.
        let d = Debounce::default()
            .push(Stamp(0), brightness(40))
            .push(Stamp(50), brightness(45))
            .push(Stamp(60), volume)
            .push(Stamp(90), other_display)
            .push(Stamp(100), brightness(50));
        assert_eq!(d.next_due(), Some(Stamp(180)));

        let cases: &[(u64, &[Write])] = &[
            (119, &[]),
            (179, &[]),
            (180, &[volume]),
            (215, &[other_display]),
            (220, &[brightness(50)]),
            (500, &[]),
        ];
        let rest = cases.iter().fold(d, |d, &(at, want)| {
            let (rest, due) = d.take_due(Stamp(at));
            assert_eq!(due, want, "at {at} ms");
            rest
        });
        assert_eq!(rest.next_due(), None);
    }

    #[test]
    fn all_due_writes_come_out_together_in_push_order() {
        let volume = Write {
            feature: Feature::SpeakerVolume,
            ..brightness(20)
        };
        let d = Debounce::default()
            .push(Stamp(0), volume)
            .push(Stamp(10), brightness(40));
        let (rest, due) = d.take_due(Stamp(1000));
        assert_eq!(due, vec![volume, brightness(40)]);
        assert_eq!(rest, Debounce::default());
    }
}
