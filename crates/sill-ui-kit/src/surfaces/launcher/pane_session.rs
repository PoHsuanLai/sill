//! One opening's pane state beside its [`Session`](super::session::Session): the preview
//! pane's machine, the sections opened with "Show More", and which providers Tab visits. Pure.

use sill_launcher::ProviderKind;

use crate::links::OpenSerial;
use crate::surfaces::launcher::keys::{Filter, next_filter};
use crate::surfaces::launcher::preview::keys::PreviewState;
use crate::surfaces::launcher::view::Expanded;

/// The pane state of one opening.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PaneSession {
    pub serial: OpenSerial,
    pub preview: PreviewState,
    pub expanded: Expanded,
    pub ring: Ring,
}

/// Which providers Tab visits: every one (nothing typed: a provider asked alone may list what
/// it has for nothing typed, the emoji grid), or those that showed a row for the typed text.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Ring {
    #[default]
    Every,
    Showing(Vec<ProviderKind>),
}

impl PaneSession {
    /// This state when it is `serial`'s, else a fresh one.
    pub fn for_serial(&self, serial: OpenSerial) -> PaneSession {
        if self.serial == serial {
            self.clone()
        } else {
            PaneSession {
                serial,
                ..PaneSession::default()
            }
        }
    }
}

/// The next filter from `filter` that `ring` visits, stepping with `next`; `Filter::All` when
/// none does.
pub fn next_in_ring(filter: Filter, ring: &Ring, next: impl Fn(Filter) -> Filter) -> Filter {
    let mut candidate = next(filter);
    for _ in 0..=ProviderKind::ALL.len() {
        match (candidate, ring) {
            (Filter::All, _) | (Filter::Only(_), Ring::Every) => return candidate,
            (Filter::Only(kind), Ring::Showing(kinds)) if kinds.contains(&kind) => {
                return candidate;
            }
            (Filter::Only(_), Ring::Showing(_)) => candidate = next(candidate),
        }
    }
    Filter::All
}

/// Tab's ring for an opening with `query` typed whose rows come from `showing` providers.
pub fn ring_for(query: &str, showing: Vec<ProviderKind>) -> Ring {
    match query.trim() {
        "" => Ring::Every,
        _ => Ring::Showing(showing),
    }
}

/// [`next_in_ring`] forwards, for a test.
pub fn tab(filter: Filter, ring: &Ring) -> Filter {
    next_in_ring(filter, ring, next_filter)
}

#[cfg(test)]
mod tests {
    use super::{PaneSession, Ring, ring_for, tab};
    use crate::links::OpenSerial;
    use crate::surfaces::launcher::keys::Filter;
    use sill_launcher::ProviderKind;

    #[test]
    fn tab_visits_the_ring_and_every_provider_with_nothing_typed() {
        use ProviderKind::{Apps, Emoji, Files};
        let ring = ring_for("m9", vec![Apps, Files]);
        assert_eq!(tab(Filter::All, &ring), Filter::Only(Apps));
        assert_eq!(tab(Filter::Only(Apps), &ring), Filter::Only(Files));
        assert_eq!(tab(Filter::Only(Files), &ring), Filter::All);
        assert_eq!(tab(Filter::All, &Ring::Showing(Vec::new())), Filter::All);
        let every = ring_for("  ", vec![Apps]);
        assert_eq!(every, Ring::Every);
        assert_eq!(
            tab(Filter::Only(Files), &every),
            Filter::Only(ProviderKind::Clipboard)
        );
        assert_eq!(
            tab(Filter::Only(ProviderKind::Clipboard), &every),
            Filter::Only(Emoji)
        );
    }

    #[test]
    fn a_new_opening_starts_over() {
        let old = PaneSession {
            serial: OpenSerial(1),
            ring: Ring::Showing(vec![ProviderKind::Apps]),
            ..PaneSession::default()
        };
        assert_eq!(old.for_serial(OpenSerial(1)), old);
        assert_eq!(old.for_serial(OpenSerial(2)).ring, Ring::Every);
    }
}
