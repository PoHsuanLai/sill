//! Which index `launcher.files_backend` picks on this machine. [`pick`] is pure; [`detect`]
//! only asks whether each backend exists and runs (a bus name's owner, a program on `PATH`),
//! never what it holds.
//!
//! `Auto` never starts an indexer: it asks Tracker only while Tracker is running, else it
//! walks. A backend named in
//! the setting may start its indexer, because the person chose it.

use std::path::Path;

use sill_model::results::FilesChoice;
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

/// The index `choice` asks on a machine with `detected`, or none. `Auto` asks a running
/// Tracker, else `fd`, else `plocate`, and never starts an indexer; a
/// named backend that is missing is none (the setting said which one, and a silent substitute
/// would surprise) and one that is dormant is asked (and may start); `Walk` is `fd` else
/// `plocate`.
pub fn pick(choice: FilesChoice, detected: &Detected) -> Option<IndexKind> {
    candidates(choice)
        .iter()
        .copied()
        .find(|kind| usable(choice, detected, *kind) == Presence::Present)
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

/// The backends `choice` may use, in order of preference.
fn candidates(choice: FilesChoice) -> &'static [IndexKind] {
    match choice {
        FilesChoice::Auto => &[IndexKind::Tracker, IndexKind::Fd, IndexKind::Plocate],
        FilesChoice::Tracker => &[IndexKind::Tracker],
        FilesChoice::Walk => &[IndexKind::Fd, IndexKind::Plocate],
        FilesChoice::Off => &[],
    }
}

/// Whether `choice` may ask `kind` on this machine.
fn usable(choice: FilesChoice, detected: &Detected, kind: IndexKind) -> Presence {
    match kind {
        IndexKind::Tracker => allowed(choice, detected.tracker),
        IndexKind::Fd => detected.fd,
        IndexKind::Plocate => detected.plocate,
    }
}

/// An indexer in `state` is asked by `Auto` only while running, by its own name while there.
fn allowed(choice: FilesChoice, state: IndexState) -> Presence {
    match (choice, state) {
        (_, IndexState::Running) => Presence::Present,
        (FilesChoice::Auto, IndexState::Dormant) => Presence::Absent,
        (_, IndexState::Dormant) => Presence::Present,
        (_, IndexState::Absent) => Presence::Absent,
    }
}

#[cfg(test)]
mod tests {
    use super::IndexState::{self, Absent, Dormant, Running};
    use super::{Detected, pick};
    use crate::providers::files::index::IndexKind;
    use sill_model::results::FilesChoice;
    use sill_model::units::Presence::{self, Absent as No, Present as Yes};

    fn machine(tracker: IndexState, fd: Presence, plocate: Presence) -> Detected {
        Detected {
            tracker,
            fd,
            plocate,
        }
    }

    #[test]
    fn auto_never_starts_an_indexer_and_a_named_backend_is_never_substituted() {
        use FilesChoice::{Auto, Off, Tracker, Walk};
        let all = machine(Running, Yes, Yes);
        let cases: Vec<(FilesChoice, Detected, Option<IndexKind>)> = vec![
            (Auto, all, Some(IndexKind::Tracker)),
            (Auto, machine(Dormant, Yes, Yes), Some(IndexKind::Fd)),
            (Auto, machine(Dormant, No, Yes), Some(IndexKind::Plocate)),
            (Auto, machine(Absent, Yes, Yes), Some(IndexKind::Fd)),
            (Auto, machine(Dormant, No, No), None),
            (Auto, machine(Absent, No, No), None),
            (
                Tracker,
                machine(Dormant, Yes, Yes),
                Some(IndexKind::Tracker),
            ),
            (Tracker, machine(Absent, Yes, Yes), None),
            (Walk, all, Some(IndexKind::Fd)),
            (Walk, machine(Running, No, Yes), Some(IndexKind::Plocate)),
            (Walk, machine(Running, No, No), None),
            (Off, all, None),
        ];
        for (choice, detected, want) in cases {
            assert_eq!(pick(choice, &detected), want, "{choice:?} {detected:?}");
        }
    }
}
