//! Which index the files provider asks on this machine. [`pick`] is pure; [`detect`] only asks
//! whether each backend exists and runs (a bus name's owner, a program on `PATH`), never what it
//! holds.
//!
//! It never starts an indexer: it asks Tracker only while Tracker is running, else it walks
//! (`fd`, then `plocate`).

use std::path::Path;

use sill_model::units::Presence;

use super::index::IndexKind;
use super::program::{FD_NAMES, on_path_any};
use super::tracker::tracker_state;
use sill_services::env::Bus;

/// Whether an indexer is there, and whether asking it would start something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexState {
    /// Running now: asking it starts nothing.
    Running,
    /// Installed and enabled but not running: Tracker's miner is activatable on the bus (a
    /// query would start localsearch, which then indexes by its own settings).
    Dormant,
    /// Not installed, or disabled.
    Absent,
}

/// Which backends this machine has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Detected {
    /// `org.freedesktop.Tracker3.Miner.Files`: `Running` when it has an owner on the session
    /// bus, `Dormant` when it is only activatable (localsearch).
    pub tracker: IndexState,
    /// `fd`, or Debian's `fdfind`.
    pub fd: Presence,
    pub plocate: Presence,
}

/// The index asked on a machine with `detected`, or none: a running Tracker, else `fd`, else
/// `plocate`. A dormant Tracker is not asked (a query would start its indexer).
pub fn pick(detected: &Detected) -> Option<IndexKind> {
    [IndexKind::Tracker, IndexKind::Fd, IndexKind::Plocate]
        .into_iter()
        .find(|kind| usable(detected, *kind) == Presence::Present)
}

/// What this machine has. Blocking (a bus call, `PATH` lookups): the
/// provider asks once, off the UI thread, and caches it.
pub fn detect(session: &Bus) -> Detected {
    let search_path = std::env::var_os("PATH").unwrap_or_default();
    let program = |names: &[&str]| on_path_any(names, &search_path, Path::is_file);
    Detected {
        tracker: tracker_state(session),
        fd: program(FD_NAMES),
        plocate: program(&["plocate"]),
    }
}

/// Whether `kind` may be asked on this machine.
fn usable(detected: &Detected, kind: IndexKind) -> Presence {
    match kind {
        IndexKind::Tracker => match detected.tracker {
            IndexState::Running => Presence::Present,
            IndexState::Dormant | IndexState::Absent => Presence::Absent,
        },
        IndexKind::Fd => detected.fd,
        IndexKind::Plocate => detected.plocate,
    }
}

#[cfg(test)]
mod tests {
    use super::IndexState::{self, Absent, Dormant, Running};
    use super::{Detected, pick};
    use crate::providers::files::index::IndexKind;
    use sill_model::units::Presence::{self, Absent as No, Present as Yes};

    fn machine(tracker: IndexState, fd: Presence, plocate: Presence) -> Detected {
        Detected {
            tracker,
            fd,
            plocate,
        }
    }

    #[test]
    fn it_never_starts_an_indexer_and_walks_when_tracker_is_not_running() {
        let all = machine(Running, Yes, Yes);
        let cases: Vec<(Detected, Option<IndexKind>)> = vec![
            (all, Some(IndexKind::Tracker)),
            (machine(Dormant, Yes, Yes), Some(IndexKind::Fd)),
            (machine(Dormant, No, Yes), Some(IndexKind::Plocate)),
            (machine(Absent, Yes, Yes), Some(IndexKind::Fd)),
            (machine(Dormant, No, No), None),
            (machine(Absent, No, No), None),
        ];
        for (detected, want) in cases {
            assert_eq!(pick(&detected), want, "{detected:?}");
        }
    }
}
