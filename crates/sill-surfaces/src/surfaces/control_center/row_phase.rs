//! Which row an operation is on, and its `RowPhase` (design/26 5.2.2 G16-G18, 5.2.3 G20-G21),
//! pure over the service's attempts (`Network.join`, `Bluetooth.connect`, sill `attempt.rs`) and
//! the row's own item.
//!
//! The services stamp one attempt at a time and do not say which item it is for; the row whose
//! item reports `Connecting` is the one. A pane remembers the last item it saw connecting
//! ([`aim_after`]), so the failure that follows (its item back to idle) and the success (its
//! item in use) land on that row. A row that never saw its `Pending` (the pane opened after the
//! join) rests: quire plays a plain Change for a Succeeded nobody watched start, and no Change
//! reads the same.

use ds::RowPhase;
use ds::detail::EventStamp;
use sill_model::attempt::{Attempt, Attempts, OpStamp};

/// Where a row's item stands with the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Item {
    Idle,
    /// Joining or connecting: the attempt under way is this row's.
    Connecting,
    /// The network or device in use.
    InUse,
}

/// Whether a row's item is the last one the pane saw connecting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Aim {
    Last,
    Other,
}

/// The last item a pane saw connecting, after seeing `connecting` (the item connecting now, if
/// any): it replaces the one before; with none, the one before stays.
pub(crate) fn aim_after<K>(last: Option<K>, connecting: Option<K>) -> Option<K> {
    connecting.or(last)
}

/// A row's phase: pending while its item connects under a running attempt, failed with the
/// failure's stamp when it was the last to connect, succeeded with the landed attempt's stamp
/// when it was the last to connect and is now in use, and at rest otherwise.
pub(crate) fn row_phase(attempts: Attempts, item: Item, aim: Aim) -> RowPhase {
    match (attempts.now, item, aim) {
        (Attempt::Running(stamp), Item::Connecting, _) => RowPhase::Pending(event(stamp)),
        (Attempt::Failed(stamp), Item::Idle | Item::Connecting, Aim::Last) => {
            RowPhase::Failed(event(stamp))
        }
        (Attempt::Quiet, Item::InUse, Aim::Last) => RowPhase::Succeeded(event(attempts.minted)),
        (Attempt::Running(_), Item::Idle | Item::InUse, _)
        | (Attempt::Failed(_), Item::InUse, _)
        | (Attempt::Failed(_), Item::Idle | Item::Connecting, Aim::Other)
        | (Attempt::Quiet, Item::Idle | Item::Connecting, _)
        | (Attempt::Quiet, Item::InUse, Aim::Other) => RowPhase::Rest,
    }
}

/// A service stamp as quire's.
fn event(stamp: OpStamp) -> EventStamp {
    EventStamp(stamp.0)
}

/// The words a failed row's detail line says (R8): the services report no reason yet.
pub(crate) const FAILED_WORDS: &str = "Couldn’t connect";

#[cfg(test)]
mod tests {
    use super::{Aim, Item, aim_after, row_phase};
    use ds::RowPhase;
    use ds::detail::EventStamp;
    use sill_model::attempt::{Attempt, Attempts, OpStamp};

    fn at(now: Attempt, minted: u32) -> Attempts {
        Attempts {
            now,
            minted: OpStamp(minted),
        }
    }

    #[test]
    fn a_rows_phase_follows_its_items_attempt() {
        use Attempt::{Failed, Quiet, Running};
        let s = OpStamp;
        let e = EventStamp;
        // (name, attempts, item, aim, phase)
        let cases = [
            (
                "connecting under a running attempt",
                at(Running(s(1)), 1),
                Item::Connecting,
                Aim::Last,
                RowPhase::Pending(e(1)),
            ),
            (
                "another row's attempt",
                at(Running(s(1)), 1),
                Item::Idle,
                Aim::Other,
                RowPhase::Rest,
            ),
            (
                "the network in use while another joins",
                at(Running(s(1)), 1),
                Item::InUse,
                Aim::Other,
                RowPhase::Rest,
            ),
            (
                "landed on this row",
                at(Quiet, 1),
                Item::InUse,
                Aim::Last,
                RowPhase::Succeeded(e(1)),
            ),
            (
                "in use, never seen joining",
                at(Quiet, 1),
                Item::InUse,
                Aim::Other,
                RowPhase::Rest,
            ),
            (
                "failed on this row",
                at(Failed(s(2)), 2),
                Item::Idle,
                Aim::Last,
                RowPhase::Failed(e(2)),
            ),
            (
                "failed on another row",
                at(Failed(s(2)), 2),
                Item::Idle,
                Aim::Other,
                RowPhase::Rest,
            ),
            (
                "a failure beside a connected device",
                at(Failed(s(2)), 2),
                Item::InUse,
                Aim::Last,
                RowPhase::Rest,
            ),
            (
                "connecting with no attempt stamped",
                at(Quiet, 0),
                Item::Connecting,
                Aim::Last,
                RowPhase::Rest,
            ),
            (
                "idle at rest",
                at(Quiet, 3),
                Item::Idle,
                Aim::Last,
                RowPhase::Rest,
            ),
        ];
        for (name, attempts, item, aim, want) in cases {
            assert_eq!(row_phase(attempts, item, aim), want, "{name}");
        }
    }

    #[test]
    fn the_aim_is_the_last_item_seen_connecting() {
        let cases = [
            (None, None, None),
            (None, Some("home"), Some("home")),
            (Some("home"), None, Some("home")),
            (Some("home"), Some("office"), Some("office")),
        ];
        for (last, connecting, want) in cases {
            assert_eq!(aim_after(last, connecting), want, "{last:?} {connecting:?}");
        }
    }
}
