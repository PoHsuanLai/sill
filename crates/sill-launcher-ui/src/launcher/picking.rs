//! Whether a pick is under way. The palette calls `onclose` just before `onpick` in one handler,
//! and the launcher must stay mapped until the pick has its activation token; so a request to
//! close is held back for a short while after a pick starts, and never beyond it: Escape and a
//! press outside always close the launcher, whatever the pick is still waiting for.

use std::time::{Duration, Instant};

/// How long after a pick starts a request to close is held back. The activation token has its
/// own, shorter deadline (`sill_ui_kit::surface::ACTIVATION_DEADLINE`), so by now the pick has
/// what it needed.
pub(crate) const PICK_SETTLE: Duration = Duration::from_secs(1);

/// Whether a pick is under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Picking {
    No,
    Yes { since: Instant },
}

impl Picking {
    /// A pick that starts at `now`.
    pub(crate) fn starting(now: Instant) -> Picking {
        Picking::Yes { since: now }
    }

    /// Whether a request to close at `now` is held back: a pick began less than [`PICK_SETTLE`]
    /// ago.
    pub(crate) fn holds_close(self, now: Instant) -> bool {
        match self {
            Picking::No => false,
            Picking::Yes { since } => now.saturating_duration_since(since) < PICK_SETTLE,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{PICK_SETTLE, Picking};

    #[test]
    fn a_close_is_held_back_only_while_a_pick_is_settling() {
        let begun = Instant::now();
        let at = |after: Duration| begun + after;
        let pick = Picking::starting(begun);
        let cases = [
            ("no pick", Picking::No, at(Duration::ZERO), false),
            ("the same instant", pick, at(Duration::ZERO), true),
            (
                "just before it settles",
                pick,
                at(PICK_SETTLE - Duration::from_millis(1)),
                true,
            ),
            ("settled", pick, at(PICK_SETTLE), false),
            (
                "long after, an ask still in flight",
                pick,
                at(Duration::from_secs(30)),
                false,
            ),
        ];
        for (name, picking, now, want) in cases {
            assert_eq!(picking.holds_close(now), want, "{name}");
        }
    }
}
