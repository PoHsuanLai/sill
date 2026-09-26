//! Which index `launcher.files_backend` picks on this machine. [`pick`] is pure; [`detect`]
//! only asks whether each backend exists (a bus name's owner, a program on `PATH`), never
//! what it holds.

use crate::results::FilesChoice;
use crate::units::Presence;

use super::index::IndexKind;

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
    let _ = (choice, detected);
    todo!("M9 fill: a-files")
}

/// What this machine has. Blocking (a bus call and `PATH` lookups): the provider asks once,
/// off the UI thread, and caches it.
pub fn detect() -> Detected {
    todo!("M9 fill: a-files")
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
    #[ignore = "M9 fill: a-files"]
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
            (FilesChoice::Auto, only(Absent, Absent, Absent, Absent), None),
            (FilesChoice::Baloo, ALL, Some(IndexKind::Baloo)),
            (FilesChoice::Tracker, only(Absent, Present, Present, Present), None),
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
