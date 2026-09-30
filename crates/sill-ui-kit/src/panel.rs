//! The panel a column surface hangs in: the notification center and Edit Widgets share its
//! corner and padding, and its life on one output, pure. A link says whether the column is wanted
//! here; it enters, stays, plays its exit and only then is the surface unmapped (the link has no
//! "leaving" state, so the exit is the surface's own to count).

/// The panel's corner: quire's Popover (`--r-panel`, 14).
pub const PANEL_RADIUS: i32 = 14;

/// quire's `SidePanel` pads its content by `--s-12` (Edit Widgets' frame does the same): the column inside is that much smaller.
pub const PANEL_PADDING: i32 = 12;

/// One opening: each one remounts the column, so its entrance plays and its focus and opened
/// groups start over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Opening(pub u64);

/// Where the column is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Unmapped; the last opening, so the next counts on.
    Hidden(Opening),
    /// Mapped, entering or entered.
    Up(Opening),
    /// Mapped, playing its exit.
    Leaving(Opening),
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Hidden(Opening::default())
    }
}

impl Phase {
    /// The opening drawn, while mapped.
    pub fn drawn(self) -> Option<Opening> {
        match self {
            Phase::Hidden(_) => None,
            Phase::Up(opening) | Phase::Leaving(opening) => Some(opening),
        }
    }
}

/// Whether the link wants the center on this output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Wanted {
    Here,
    NotHere,
}

/// What moves the column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhaseInput {
    Link(Wanted),
    /// The exit's timer ran out.
    ExitSettled,
}

/// What the surface must do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhaseOut {
    Map,
    PlayExit,
    Unmap,
}

/// `phase` after `input`. Wanted again while leaving, the column comes back without a new
/// opening (no second entrance); a settle that finds the column up is a stale timer.
pub fn step_phase(phase: Phase, input: PhaseInput) -> (Phase, Option<PhaseOut>) {
    use PhaseInput::{ExitSettled, Link};
    use Wanted::{Here, NotHere};
    match (phase, input) {
        (Phase::Hidden(Opening(n)), Link(Here)) => (Phase::Up(Opening(n + 1)), Some(PhaseOut::Map)),
        (Phase::Up(opening), Link(NotHere)) => (Phase::Leaving(opening), Some(PhaseOut::PlayExit)),
        (Phase::Leaving(opening), Link(Here)) => (Phase::Up(opening), None),
        (Phase::Leaving(opening), ExitSettled) => (Phase::Hidden(opening), Some(PhaseOut::Unmap)),
        (Phase::Hidden(_), Link(NotHere) | ExitSettled)
        | (Phase::Up(_), Link(Here) | ExitSettled)
        | (Phase::Leaving(_), Link(NotHere)) => (phase, None),
    }
}

#[cfg(test)]
mod tests {
    use super::{Opening, Phase, PhaseInput, PhaseOut, Wanted, step_phase};

    #[test]
    fn the_column_enters_leaves_and_unmaps_after_its_exit() {
        use Phase::{Hidden, Leaving, Up};
        use PhaseInput::{ExitSettled, Link};
        use Wanted::{Here, NotHere};
        let (one, two) = (Opening(1), Opening(2));
        let cases = [
            (Hidden(Opening(0)), Link(Here), Up(one), Some(PhaseOut::Map)),
            (Hidden(one), Link(Here), Up(two), Some(PhaseOut::Map)),
            (Hidden(one), Link(NotHere), Hidden(one), None),
            (Hidden(one), ExitSettled, Hidden(one), None),
            (Up(one), Link(Here), Up(one), None),
            (
                Up(one),
                Link(NotHere),
                Leaving(one),
                Some(PhaseOut::PlayExit),
            ),
            (Up(one), ExitSettled, Up(one), None),
            (Leaving(one), Link(Here), Up(one), None),
            (Leaving(one), Link(NotHere), Leaving(one), None),
            (
                Leaving(one),
                ExitSettled,
                Hidden(one),
                Some(PhaseOut::Unmap),
            ),
        ];
        for (phase, input, want, out) in cases {
            assert_eq!(step_phase(phase, input), (want, out), "{phase:?} {input:?}");
        }
        assert_eq!(Hidden(one).drawn(), None);
        assert_eq!(Leaving(two).drawn(), Some(two));
    }
}
