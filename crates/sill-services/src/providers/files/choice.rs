//! Which index `launcher.files_backend` picks on this machine. [`pick`] is pure; [`detect`]
//! only asks whether each backend exists (a bus name's owner, a program on `PATH`), never
//! what it holds.

use std::path::Path;

use crate::results::FilesChoice;
use crate::units::Presence;

use super::baloo::baloo_enabled;
use super::index::IndexKind;
use super::program::on_path;
use super::tracker::tracker_presence;

/// Which backends this machine has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Detected {
    /// `org.freedesktop.Tracker3.Miner.Files` has an owner or is activatable on the session
    /// bus (localsearch).
    pub tracker: Presence,
    /// `baloosearch6` is on `PATH` and Baloo is enabled (`balooctl6 status` says so).
    pub baloo: Presence,
    pub fd: Presence,
    pub plocate: Presence,
}

/// The index `choice` asks on a machine with `detected`, or none. `Auto` prefers Tracker, then
/// Baloo, then `fd`, then `plocate`; a named backend that is missing is none (the setting said
/// which one, and a silent substitute would surprise), except `Walk`, which is `fd` else
/// `plocate`.
pub fn pick(choice: FilesChoice, detected: &Detected) -> Option<IndexKind> {
    candidates(choice)
        .iter()
        .copied()
        .find(|kind| presence_of(detected, *kind) == Presence::Present)
}

/// What this machine has. Blocking (a bus call and `PATH` lookups): the provider asks once,
/// off the UI thread, and caches it.
pub fn detect() -> Detected {
    let search_path = std::env::var_os("PATH").unwrap_or_default();
    let program = |name: &str| on_path(name, &search_path, Path::is_file);
    let baloo = match program("baloosearch6") {
        Presence::Present => baloo_enabled(),
        Presence::Absent => Presence::Absent,
    };
    Detected {
        tracker: tracker_presence(),
        baloo,
        fd: program("fd"),
        plocate: program("plocate"),
    }
}

/// The backends `choice` may use, in order of preference.
fn candidates(choice: FilesChoice) -> &'static [IndexKind] {
    match choice {
        FilesChoice::Auto => &[
            IndexKind::Tracker,
            IndexKind::Baloo,
            IndexKind::Fd,
            IndexKind::Plocate,
        ],
        FilesChoice::Tracker => &[IndexKind::Tracker],
        FilesChoice::Baloo => &[IndexKind::Baloo],
        FilesChoice::Walk => &[IndexKind::Fd, IndexKind::Plocate],
        FilesChoice::Off => &[],
    }
}

fn presence_of(detected: &Detected, kind: IndexKind) -> Presence {
    match kind {
        IndexKind::Tracker => detected.tracker,
        IndexKind::Baloo => detected.baloo,
        IndexKind::Fd => detected.fd,
        IndexKind::Plocate => detected.plocate,
        IndexKind::Fake => Presence::Absent,
    }
}

#[cfg(test)]
mod tests {
    use super::{Detected, pick};
    use crate::providers::files::index::IndexKind;
    use crate::results::FilesChoice;
    use crate::units::Presence::{Absent, Present};

    const ALL: Detected = Detected {
        tracker: Present,
        baloo: Present,
        fd: Present,
        plocate: Present,
    };

    #[test]
    fn auto_prefers_an_index_and_a_named_backend_is_never_substituted() {
        let only = |tracker, baloo, fd, plocate| Detected {
            tracker,
            baloo,
            fd,
            plocate,
        };
        let cases: Vec<(FilesChoice, Detected, Option<IndexKind>)> = vec![
            (FilesChoice::Auto, ALL, Some(IndexKind::Tracker)),
            (
                FilesChoice::Auto,
                only(Absent, Present, Present, Present),
                Some(IndexKind::Baloo),
            ),
            (
                FilesChoice::Auto,
                only(Absent, Absent, Absent, Present),
                Some(IndexKind::Plocate),
            ),
            (
                FilesChoice::Auto,
                only(Absent, Absent, Absent, Absent),
                None,
            ),
            (FilesChoice::Baloo, ALL, Some(IndexKind::Baloo)),
            (
                FilesChoice::Tracker,
                only(Absent, Present, Present, Present),
                None,
            ),
            (FilesChoice::Walk, ALL, Some(IndexKind::Fd)),
            (
                FilesChoice::Walk,
                only(Present, Present, Absent, Present),
                Some(IndexKind::Plocate),
            ),
            (FilesChoice::Off, ALL, None),
        ];
        for (choice, detected, want) in cases {
            assert_eq!(pick(choice, &detected), want, "{choice:?} {detected:?}");
        }
    }
}
