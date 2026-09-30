//! The preview pane's moments through quire's `ds::detail` (design/26-DETAILS.md): its states as
//! the person sees them, what each change means (an exhaustive [`Detailed`] table), and
//! [`use_pane_detail`], the one place the pane calls `use_detail`. The launcher never plays a
//! moment itself (quire rule 4): the cue goes to quire's `PreviewPane` (`cue`, and `operation`
//! from `use_operation`), which plays it.

use ds::detail::{Detail, Touch};
use ds::prelude::*;
use sill_launcher::PreviewKind;

/// The pane as the person sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneState {
    /// Leaving, or not there.
    Hidden,
    /// Showing a preview of this kind, all of it loaded.
    Showing(PreviewKind),
    /// Waiting on a slow load: a text file's head, a PDF's page.
    Pending,
    /// What it names could not be read: the still words say so.
    Failed,
}

impl Detailed for PaneState {
    fn moment(from: &Self, to: &Self) -> Moment {
        use PaneState::{Failed, Hidden, Pending, Showing};
        match (from, to) {
            (Hidden, Hidden) | (Pending, Pending) | (Failed, Failed) => Moment::Rest,
            (Hidden, Showing(_) | Failed) => Moment::Appear,
            // The selection moved (instant, design/13 §13.3.9): the content changes in place,
            // the pane's entrance does not replay. The same kind is the same pane (R2).
            (Showing(was), Showing(now)) if was == now => Moment::Rest,
            (Showing(_) | Failed, Showing(_)) | (Showing(_), Failed) => Moment::Preview,
            (Hidden | Showing(_) | Failed, Pending) => Moment::Pending,
            // A load landing is content arriving, not an operation the person asked to succeed:
            // no check, no seal (R12).
            (Pending, Showing(_)) => Moment::Change,
            (Pending, Failed) => Moment::Failure,
            (Showing(_) | Pending | Failed, Hidden) => Moment::Dismiss,
        }
    }

    fn first(state: &Self) -> Moment {
        match state {
            PaneState::Hidden => Moment::Rest,
            PaneState::Showing(_) | PaneState::Failed => Moment::Appear,
            PaneState::Pending => Moment::Pending,
        }
    }
}

/// Track the pane's `state`, caused by `touch` (a key or a press in the panel, else
/// `Touch::Remote`). The pane mounts when the person shows it, so its first frame animates.
pub fn use_pane_detail(state: PaneState, touch: Touch) -> Detail<PaneState> {
    use_detail(state, touch)
}

#[cfg(test)]
mod tests {
    use super::PaneState;
    use super::PaneState::{Failed, Hidden, Pending, Showing};
    use ds::detail::{first_table, moment_table};
    use ds::prelude::*;
    use sill_launcher::PreviewKind::{Image, Pdf, TextFile};

    #[test]
    fn the_pane_moments() {
        #[rustfmt::skip]
        moment_table::<PaneState>(&[
            (Hidden, Showing(Image), Moment::Appear),
            (Hidden, Pending, Moment::Pending),
            (Hidden, Failed, Moment::Appear),
            (Showing(Image), Showing(Pdf), Moment::Preview),
            (Showing(Image), Showing(Image), Moment::Rest),
            (Showing(Image), Pending, Moment::Pending),
            (Showing(Image), Failed, Moment::Preview),
            (Pending, Showing(TextFile), Moment::Change),
            (Pending, Failed, Moment::Failure),
            (Pending, Pending, Moment::Rest),
            (Failed, Showing(Image), Moment::Preview),
            (Failed, Pending, Moment::Pending),
            (Showing(Pdf), Hidden, Moment::Dismiss),
            (Pending, Hidden, Moment::Dismiss),
            (Failed, Hidden, Moment::Dismiss),
            (Hidden, Hidden, Moment::Rest),
        ]);
    }

    #[test]
    fn the_pane_first_frames() {
        first_table::<PaneState>(&[
            (Hidden, Moment::Rest),
            (Showing(Image), Moment::Appear),
            (Pending, Moment::Pending),
            (Failed, Moment::Appear),
        ]);
    }
}
