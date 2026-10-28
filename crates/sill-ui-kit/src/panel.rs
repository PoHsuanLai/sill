//! The panel a column surface hangs in: the notification center's corner and padding, and its life on one output, pure. A link says whether the column is wanted
//! here; it enters, stays, plays its exit and only then is the surface unmapped (the link has no
//! "leaving" state, so the exit is the surface's own to count).

/// The panel's corner: quire's Popover (`--r-panel`, 14).
pub const PANEL_RADIUS: i32 = 14;

/// quire's `SidePanel` pads its content by `--s-12`: the column inside is that much smaller.
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

/// What moves the column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhaseInput {
    /// The link wants the center on this output.
    Wanted,
    /// The link wants it elsewhere, or nowhere.
    NotWanted,
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
    use PhaseInput::{ExitSettled, NotWanted, Wanted};
    match (phase, input) {
        (Phase::Hidden(Opening(n)), Wanted) => (Phase::Up(Opening(n + 1)), Some(PhaseOut::Map)),
        (Phase::Up(opening), NotWanted) => (Phase::Leaving(opening), Some(PhaseOut::PlayExit)),
        (Phase::Leaving(opening), Wanted) => (Phase::Up(opening), None),
        (Phase::Leaving(opening), ExitSettled) => (Phase::Hidden(opening), Some(PhaseOut::Unmap)),
        (Phase::Hidden(_), NotWanted | ExitSettled)
        | (Phase::Up(_), Wanted | ExitSettled)
        | (Phase::Leaving(_), NotWanted) => (phase, None),
    }
}

#[cfg(test)]
mod tests {
    use super::{Opening, Phase, PhaseInput, PhaseOut, step_phase};

    #[test]
    fn the_column_enters_leaves_and_unmaps_after_its_exit() {
        use Phase::{Hidden, Leaving, Up};
        use PhaseInput::{ExitSettled, NotWanted, Wanted};
        let (one, two) = (Opening(1), Opening(2));
        let cases = [
            (Hidden(Opening(0)), Wanted, Up(one), Some(PhaseOut::Map)),
            (Hidden(one), Wanted, Up(two), Some(PhaseOut::Map)),
            (Hidden(one), NotWanted, Hidden(one), None),
            (Hidden(one), ExitSettled, Hidden(one), None),
            (Up(one), Wanted, Up(one), None),
            (Up(one), NotWanted, Leaving(one), Some(PhaseOut::PlayExit)),
            (Up(one), ExitSettled, Up(one), None),
            (Leaving(one), Wanted, Up(one), None),
            (Leaving(one), NotWanted, Leaving(one), None),
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
