//! One opening's panel state: the typed query, the generation the launcher opened over (older
//! results are stale), the keyboard machine's state, and how many times the palette has been
//! remounted to take the keyboard back from an overlay. Pure.

use sill_launcher::Generation;

use crate::surfaces::launcher::keys::{Filter, LauncherState};
use crate::surfaces::launcher::presence::{OpenSerial, Opening};

/// The panel's state for one opening.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Session {
    pub serial: OpenSerial,
    pub query: String,
    /// Results of this generation or older belong to an earlier keystroke or opening.
    pub since: Generation,
    pub keys: LauncherState,
    pub mounts: Remounts,
}

/// Counts the palette's remounts within an opening: the quire palette owns its field's focus
/// and selection, and remounting it is the one way to put focus back and the selection on the
/// first row (FINDINGS F141, Q43).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Remounts(pub u32);

impl Session {
    /// A fresh session for `opening`, over results of generation `since`.
    pub fn opened(opening: &Opening, since: Generation) -> Session {
        Session {
            serial: opening.serial,
            query: opening.query.clone(),
            since,
            keys: LauncherState::default(),
            mounts: Remounts::default(),
        }
    }

    /// This session when it is `opening`'s, else a fresh one. A render can run before the
    /// opening's effect stores its session; until then every generation counts as fresh.
    pub fn for_opening(&self, opening: &Opening) -> Session {
        if self.serial == opening.serial {
            self.clone()
        } else {
            Session::opened(opening, Generation::default())
        }
    }

    /// The palette's key: a new opening, filter or remount mounts a new palette (its entrance
    /// plays, its field takes focus, its selection starts at the top).
    pub fn palette_key(&self) -> String {
        let filter = match self.keys.filter {
            Filter::All => "all".to_owned(),
            Filter::Only(kind) => format!("{kind:?}"),
        };
        format!("{}-{filter}-{}", self.serial.0, self.mounts.0)
    }

    /// The palette remounted: its selection is back on the first row.
    pub fn remounted(self) -> Session {
        Session {
            mounts: Remounts(self.mounts.0 + 1),
            keys: LauncherState {
                selected: 0,
                ..self.keys
            },
            ..self
        }
    }

    /// The query changed: the palette puts its selection on the first row.
    pub fn typed(self, query: String) -> Session {
        Session {
            query,
            keys: LauncherState {
                selected: 0,
                ..self.keys
            },
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Remounts, Session};
    use crate::surfaces::launcher::keys::{Filter, LauncherState};
    use crate::surfaces::launcher::presence::{OpenSerial, Opening, PanelFocus};
    use shell_host::OutputId;
    use sill_launcher::{Generation, ProviderKind};
    use std::time::Instant;

    fn opening(serial: u64, query: &str) -> Opening {
        Opening {
            output: OutputId(1),
            query: query.to_owned(),
            serial: OpenSerial(serial),
            requested: Instant::now(),
            focus: PanelFocus::Awaited,
        }
    }

    #[test]
    fn a_new_opening_starts_over_and_keys_the_palette_anew() {
        let first = Session::opened(&opening(1, "fi"), Generation(3));
        assert_eq!(first.query, "fi");
        assert_eq!(first.since, Generation(3));
        let moved = Session {
            keys: LauncherState {
                selected: 2,
                filter: Filter::Only(ProviderKind::Apps),
                ..LauncherState::default()
            },
            ..first.clone()
        };
        assert_eq!(moved.for_opening(&opening(1, "fi")), moved, "the same opening");
        let next = moved.for_opening(&opening(2, ""));
        assert_eq!(next.keys, LauncherState::default());
        assert_eq!(next.since, Generation(0), "until the opening's effect records it");
        assert_ne!(moved.palette_key(), first.palette_key(), "a filter remounts");
        let again = moved.clone().remounted();
        assert_eq!(again.mounts, Remounts(1));
        assert_eq!(again.keys.selected, 0);
        assert_eq!(again.keys.filter, Filter::Only(ProviderKind::Apps));
        assert_ne!(again.palette_key(), moved.palette_key());
        assert_eq!(moved.typed("fir".to_owned()).keys.selected, 0);
    }
}
