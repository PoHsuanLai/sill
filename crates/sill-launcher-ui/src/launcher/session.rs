//! One opening's panel state: the typed query, the generation the launcher opened over (older
//! results are stale) and the keyboard machine's state. Pure.

use sill_launcher::{Generation, LauncherMode, ProviderKind};

use crate::launcher::keys::{Filter, LauncherState};
use sill_ui_kit::links::{OpenSerial, Opening};

/// The filter a panel starts in: every provider to search, the companion's own rows to talk.
pub fn filter_for(mode: LauncherMode) -> Filter {
    match mode {
        LauncherMode::Search => Filter::All,
        LauncherMode::Companion => Filter::Only(ProviderKind::Companion),
    }
}

/// The panel's state for one opening.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Session {
    pub serial: OpenSerial,
    pub query: String,
    /// Results of this generation or older belong to an earlier keystroke or opening.
    pub since: Generation,
    pub keys: LauncherState,
    /// The search text that was in the field when the panel became a prompt: the field empties
    /// (it is a chip of the prompt), and leaving companion mode puts it back.
    pub kept: Option<String>,
}

impl Session {
    /// A fresh session for `opening`, over results of generation `since`.
    pub fn opened(opening: &Opening, since: Generation) -> Session {
        Session {
            serial: opening.serial,
            query: opening.query.clone(),
            since,
            keys: LauncherState {
                filter: filter_for(opening.mode),
                ..LauncherState::default()
            },
            kept: None,
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
    use super::Session;
    use crate::launcher::keys::{Filter, LauncherState};
    use shell_host::OutputId;
    use sill_launcher::{Generation, LauncherMode, ProviderKind};
    use sill_ui_kit::links::{OpenSerial, Opening, PanelFocus};
    use std::time::Instant;

    fn opening(serial: u64, query: &str) -> Opening {
        Opening {
            output: OutputId(1),
            query: query.to_owned(),
            serial: OpenSerial(serial),
            mode: LauncherMode::Search,
            requested: Instant::now(),
            focus: PanelFocus::Awaited,
        }
    }

    #[test]
    fn a_companion_opening_starts_on_the_companions_rows() {
        let companion = Opening {
            mode: LauncherMode::Companion,
            ..opening(1, "")
        };
        let session = Session::opened(&companion, Generation(0));
        assert_eq!(session.keys.filter, Filter::Only(ProviderKind::Companion));
        assert_eq!(
            Session::opened(&opening(1, ""), Generation(0)).keys.filter,
            Filter::All
        );
    }

    #[test]
    fn a_new_opening_starts_over() {
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
        assert_eq!(
            moved.for_opening(&opening(1, "fi")),
            moved,
            "the same opening"
        );
        let next = moved.for_opening(&opening(2, ""));
        assert_eq!(
            next.keys,
            LauncherState::default(),
            "a search opening starts at every provider"
        );
        assert_eq!(
            next.since,
            Generation(0),
            "until the opening's effect records it"
        );
        assert_eq!(moved.typed("fir".to_owned()).keys.selected, 0);
    }
}
