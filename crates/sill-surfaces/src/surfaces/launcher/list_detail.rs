//! The results' moments through quire's `ds::detail` (design/26-DETAILS.md §5.6, §8 item 6;
//! FINDINGS "Launcher details (Q297)"): the first result set after the panel opens appears (the
//! rows' `rise` stagger, R13), later sets replace in place with no stagger (R1, R12: typing is
//! frequent), and a section's "Show More" or "Show Less" is a change the person's key or press
//! caused. The cue goes to quire's `CommandPalette { reveal }` (Q372), which rises only on an
//! Appear. Show More and Show Less have no moment of their own in `ds::detail`: design/26 §5.6
//! decided them (Q373: the added rows `Reveal` downward, Show Less `heal`s up) and the palette
//! plays them itself from its own group action, so this table keeps calling them a Change (which
//! replays no rise). Pure but for [`use_list_detail`], the one place the results call
//! `use_detail`.

use ds::detail::{Detail, Detailed, FirstShow, Moment, Touch, use_detail};
use sill_launcher::{Expanded, Generation};

use crate::surfaces::launcher::presence::OpenSerial;

/// The results as the person sees them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListState {
    /// The launcher is closed.
    Closed,
    /// Open, no answer of this opening drawn yet.
    Waiting(OpenSerial),
    /// An answer drew no rows: the still "Nothing matches" line.
    Empty(OpenSerial),
    /// An answer's rows, with the sections "Show More" opened.
    Rows(Rows),
}

/// Rows drawn in one opening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rows {
    pub opening: OpenSerial,
    /// The answer drawn: a new one is the query's, typed or filtered.
    pub answer: Generation,
    pub expanded: Expanded,
}

/// The opening a state belongs to (none while closed).
fn opening_of(state: &ListState) -> Option<OpenSerial> {
    match state {
        ListState::Closed => None,
        ListState::Waiting(opening) | ListState::Empty(opening) => Some(*opening),
        ListState::Rows(rows) => Some(rows.opening),
    }
}

impl Detailed for ListState {
    fn moment(from: &Self, to: &Self) -> Moment {
        use ListState::{Closed, Empty, Rows, Waiting};
        match (from, to) {
            (Closed, Closed) => Moment::Rest,
            (Waiting(_) | Empty(_) | Rows(_), Closed) => Moment::Dismiss,
            // The surface was just opened: its first frame (R1).
            (Closed, Waiting(_) | Empty(_) | Rows(_)) => ListState::first(to),
            (Waiting(_) | Empty(_) | Rows(_), Waiting(_) | Empty(_) | Rows(_))
                if opening_of(from) != opening_of(to) =>
            {
                ListState::first(to)
            }
            (Waiting(_), Rows(_)) => Moment::Appear,
            (Rows(was), Rows(now)) if was.answer != now.answer => Moment::Change,
            // Show More or Show Less: the palette plays its own expand and collapse (Q373).
            (Rows(was), Rows(now)) if was.expanded != now.expanded => Moment::Change,
            (Rows(_), Rows(_)) => Moment::Rest,
            (Waiting(_) | Rows(_), Empty(_)) | (Empty(_), Rows(_)) => Moment::Change,
            (Waiting(_) | Empty(_) | Rows(_), Waiting(_)) | (Empty(_), Empty(_)) => Moment::Rest,
        }
    }

    fn first(state: &Self) -> Moment {
        match state {
            ListState::Rows(_) => Moment::Appear,
            ListState::Empty(_) => Moment::Change,
            ListState::Closed | ListState::Waiting(_) => Moment::Rest,
        }
    }
}

/// Track the results' `state`, caused by `touch`: a key or press in the panel for "Show More",
/// `Touch::Remote` for an answer arriving (the panel takes the touch per render, `touch.rs`).
pub fn use_list_detail(state: ListState, touch: Touch) -> Detail<ListState> {
    use_detail(state, FirstShow::Animate, touch)
}

#[cfg(test)]
mod tests {
    use super::ListState::{Closed, Empty, Waiting};
    use super::{ListState, Rows};
    use crate::surfaces::launcher::presence::OpenSerial;
    use ds::detail::{Moment, first_table, moment_table};
    use sill_launcher::{Expanded, Generation, Section};

    fn rows(opening: u64, answer: u64, expanded: &[Section]) -> ListState {
        ListState::Rows(Rows {
            opening: OpenSerial(opening),
            answer: Generation(answer),
            expanded: Expanded(expanded.to_vec()),
        })
    }

    #[test]
    fn the_results_moments() {
        let (one, two) = (OpenSerial(1), OpenSerial(2));
        let docs = [Section::Documents];
        #[rustfmt::skip]
        moment_table::<ListState>(&[
            // The first set after opening rises in; later sets replace in place.
            (Waiting(one), rows(1, 4, &[]), Moment::Appear),
            (Closed, rows(1, 4, &[]), Moment::Appear),
            (rows(1, 4, &[]), rows(1, 5, &[]), Moment::Change),
            (rows(1, 4, &[]), rows(1, 4, &[]), Moment::Rest),
            // Show More, then Show Less.
            (rows(1, 4, &[]), rows(1, 4, &docs), Moment::Change),
            (rows(1, 4, &docs), rows(1, 4, &[]), Moment::Change),
            // Nothing matches, and back.
            (rows(1, 4, &[]), Empty(one), Moment::Change),
            (Empty(one), rows(1, 6, &[]), Moment::Change),
            (Waiting(one), Empty(one), Moment::Change),
            (Empty(one), Empty(one), Moment::Rest),
            // A new opening plays its own first frame, whatever the last one showed.
            (rows(1, 4, &[]), rows(2, 7, &[]), Moment::Appear),
            (rows(1, 4, &[]), Waiting(two), Moment::Rest),
            (Empty(one), Empty(two), Moment::Change),
            // Closing, and a render before the opening's first answer.
            (rows(1, 4, &[]), Closed, Moment::Dismiss),
            (Waiting(one), Closed, Moment::Dismiss),
            (Closed, Waiting(one), Moment::Rest),
            (Closed, Closed, Moment::Rest),
        ]);
    }

    #[test]
    fn the_results_first_frames() {
        first_table::<ListState>(&[
            (Closed, Moment::Rest),
            (Waiting(OpenSerial(1)), Moment::Rest),
            (Empty(OpenSerial(1)), Moment::Change),
            (rows(1, 1, &[]), Moment::Appear),
        ]);
    }
}
