//! One opening's M9 state beside the M4 [`Session`](super::session::Session): the preview
//! pane's machine, the sections opened with "Show More", where the field's caret is thought to
//! be, and which providers Tab visits. Pure.
//!
//! The caret is an estimate: quire's palette does not report it yet (Q299), so the panel counts
//! it from what it sees (typing puts it at the end; ←, →, Home and End move it) and the pane's
//! → reads it. A click inside the field is not seen, which is the estimate's known gap (G270).

use sill_launcher::ProviderKind;

use crate::surfaces::launcher::keys::{Filter, next_filter};
use crate::surfaces::launcher::presence::OpenSerial;
use crate::surfaces::launcher::preview::keys::{Caret, PreviewState};
use crate::surfaces::launcher::view::Expanded;

/// The M9 state of one opening.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PaneSession {
    pub serial: OpenSerial,
    pub preview: PreviewState,
    pub expanded: Expanded,
    pub caret: CaretAt,
    pub ring: Ring,
}

/// The caret's estimated place, in characters from the start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CaretAt(pub usize);

/// A caret move the field made on a key the panel let through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaretMove {
    Left,
    Right,
    Home,
    End,
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

    /// Where the caret is in `query`.
    pub fn caret_in(&self, query: &str) -> Caret {
        if self.caret.0 >= query.chars().count() {
            Caret::AtEnd
        } else {
            Caret::Inside
        }
    }
}

/// The caret after `step` in `query`.
pub fn moved_caret(at: CaretAt, step: CaretMove, query: &str) -> CaretAt {
    let end = query.chars().count();
    let at = at.0.min(end);
    CaretAt(match step {
        CaretMove::Left => at.saturating_sub(1),
        CaretMove::Right => (at + 1).min(end),
        CaretMove::Home => 0,
        CaretMove::End => end,
    })
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
    use super::{CaretAt, CaretMove, PaneSession, Ring, moved_caret, ring_for, tab};
    use crate::surfaces::launcher::keys::Filter;
    use crate::surfaces::launcher::presence::OpenSerial;
    use crate::surfaces::launcher::preview::keys::Caret;
    use sill_launcher::ProviderKind;

    #[test]
    fn the_caret_estimate_follows_the_keys_the_field_took() {
        let cases: &[(usize, CaretMove, usize)] = &[
            (4, CaretMove::Left, 3),
            (0, CaretMove::Left, 0),
            (3, CaretMove::Right, 4),
            (4, CaretMove::Right, 4),
            (2, CaretMove::Home, 0),
            (0, CaretMove::End, 4),
            (9, CaretMove::Left, 3),
        ];
        for &(from, step, want) in cases {
            assert_eq!(moved_caret(CaretAt(from), step, "naïf"), CaretAt(want), "{from} {step:?}");
        }
        let at = |n| PaneSession {
            caret: CaretAt(n),
            ..PaneSession::default()
        };
        assert_eq!(at(4).caret_in("naïf"), Caret::AtEnd);
        assert_eq!(at(3).caret_in("naïf"), Caret::Inside);
    }

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
        assert_eq!(tab(Filter::Only(Files), &every), Filter::Only(ProviderKind::Clipboard));
        assert_eq!(tab(Filter::Only(ProviderKind::Clipboard), &every), Filter::Only(Emoji));
    }

    #[test]
    fn a_new_opening_starts_over() {
        let old = PaneSession {
            serial: OpenSerial(1),
            caret: CaretAt(3),
            ..PaneSession::default()
        };
        assert_eq!(old.for_serial(OpenSerial(1)), old);
        assert_eq!(old.for_serial(OpenSerial(2)).caret, CaretAt(0));
    }
}
