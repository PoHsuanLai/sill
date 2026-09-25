//! One indicator's show, hold and fade (design/20 §1.7): a change maps it and fades it in
//! (`--t-quick --e-out`); it holds `osd.hold_ms` after the last change, then fades out
//! (`--t-move --e-exit`) and unmaps. A change while it is up restarts the hold; a change during
//! the fade takes the fade back at once (no second fade-in) and restarts the hold. Pure: time is
//! a [`Stamp`] argument, and the surface wakes the machine at [`OsdPhase::wake`].

use sill_services::Stamp;

use crate::surfaces::launcher::presence::Visible;

/// How long each timed part lasts, in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OsdTimes {
    /// After the last change, before the fade out (`osd.hold_ms`).
    pub hold_ms: u64,
    /// The fade out, settled (`--t-move` at the motion level, plus quire's frame slack).
    pub leave_ms: u64,
}

/// How the indicator is drawn while mapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OsdLook {
    /// Just mapped: the fade in plays.
    Entering,
    /// Up, with no animation (a change took a fade out back).
    Steady,
    /// The fade out plays.
    Leaving,
    /// Unmapped: drawn clear, so nothing of it shows on the frame that maps it again.
    Hidden,
}

impl OsdLook {
    /// `data-look`, which the stylesheet animates.
    pub fn slug(self) -> &'static str {
        match self {
            OsdLook::Entering => "entering",
            OsdLook::Steady => "steady",
            OsdLook::Leaving => "leaving",
            OsdLook::Hidden => "hidden",
        }
    }
}

/// Where one indicator is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OsdPhase {
    /// Unmapped; nothing runs.
    #[default]
    Hidden,
    /// Up until `until`, then the fade out starts.
    Held { look: OsdLook, until: Stamp },
    /// Fading out; unmapped at `until`.
    Leaving { until: Stamp },
}

/// What moves the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsdInput {
    /// A change to show arrived at this time.
    Changed(Stamp),
    /// The surface woke at this time (see [`OsdPhase::wake`]).
    Tick(Stamp),
}

impl OsdPhase {
    /// Whether the surface is mapped.
    pub fn visible(self) -> Visible {
        match self {
            OsdPhase::Hidden => Visible::No,
            OsdPhase::Held { .. } | OsdPhase::Leaving { .. } => Visible::Yes,
        }
    }

    /// How it is drawn.
    pub fn look(self) -> OsdLook {
        match self {
            OsdPhase::Held { look, .. } => look,
            OsdPhase::Leaving { .. } => OsdLook::Leaving,
            OsdPhase::Hidden => OsdLook::Hidden,
        }
    }

    /// When the surface must wake the machine next; none once hidden, so an idle indicator
    /// runs no timer at all.
    pub fn wake(self) -> Option<Stamp> {
        match self {
            OsdPhase::Hidden => None,
            OsdPhase::Held { until, .. } | OsdPhase::Leaving { until } => Some(until),
        }
    }
}

/// `phase` after `input`.
pub fn step(phase: OsdPhase, input: OsdInput, times: OsdTimes) -> OsdPhase {
    match (phase, input) {
        (OsdPhase::Hidden, OsdInput::Changed(now)) => OsdPhase::Held {
            look: OsdLook::Entering,
            until: now.after(times.hold_ms),
        },
        (OsdPhase::Held { look, .. }, OsdInput::Changed(now)) => OsdPhase::Held {
            look,
            until: now.after(times.hold_ms),
        },
        (OsdPhase::Leaving { .. }, OsdInput::Changed(now)) => OsdPhase::Held {
            look: OsdLook::Steady,
            until: now.after(times.hold_ms),
        },
        (OsdPhase::Held { until, .. }, OsdInput::Tick(now)) if now >= until => OsdPhase::Leaving {
            until: now.after(times.leave_ms),
        },
        (OsdPhase::Leaving { until }, OsdInput::Tick(now)) if now >= until => OsdPhase::Hidden,
        (phase, OsdInput::Tick(_)) => phase,
    }
}

#[cfg(test)]
mod tests {
    use super::{OsdInput, OsdLook, OsdPhase, OsdTimes, step};
    use crate::surfaces::launcher::presence::Visible;
    use sill_services::Stamp;

    const TIMES: OsdTimes = OsdTimes {
        hold_ms: 1500,
        leave_ms: 254,
    };

    fn held(look: OsdLook, until: u64) -> OsdPhase {
        OsdPhase::Held {
            look,
            until: Stamp(until),
        }
    }

    fn leaving(until: u64) -> OsdPhase {
        OsdPhase::Leaving {
            until: Stamp(until),
        }
    }

    fn changed(at: u64) -> OsdInput {
        OsdInput::Changed(Stamp(at))
    }

    fn tick(at: u64) -> OsdInput {
        OsdInput::Tick(Stamp(at))
    }

    #[test]
    fn show_hold_and_fade_follow_the_table() {
        let cases: &[(&str, OsdPhase, OsdInput, OsdPhase)] = &[
            (
                "a change maps it, fading in",
                OsdPhase::Hidden,
                changed(100),
                held(OsdLook::Entering, 1600),
            ),
            (
                "another change restarts the hold",
                held(OsdLook::Entering, 1600),
                changed(900),
                held(OsdLook::Entering, 2400),
            ),
            (
                "an early wake changes nothing",
                held(OsdLook::Entering, 1600),
                tick(1599),
                held(OsdLook::Entering, 1600),
            ),
            (
                "the hold ends: the fade out starts",
                held(OsdLook::Entering, 1600),
                tick(1600),
                leaving(1854),
            ),
            (
                "a late wake fades from when it woke",
                held(OsdLook::Steady, 1600),
                tick(1700),
                leaving(1954),
            ),
            (
                "a change during the fade takes it back",
                leaving(1854),
                changed(1700),
                held(OsdLook::Steady, 3200),
            ),
            (
                "mid fade, nothing yet",
                leaving(1854),
                tick(1800),
                leaving(1854),
            ),
            (
                "the fade settles: unmapped",
                leaving(1854),
                tick(1854),
                OsdPhase::Hidden,
            ),
            (
                "a wake while hidden",
                OsdPhase::Hidden,
                tick(5000),
                OsdPhase::Hidden,
            ),
        ];
        for (name, before, input, want) in cases {
            assert_eq!(step(*before, *input, TIMES), *want, "{name}");
        }
    }

    #[test]
    fn only_a_hidden_indicator_is_unmapped_and_timerless() {
        const CASES: &[(OsdPhase, Visible, Option<u64>, OsdLook)] = &[
            (OsdPhase::Hidden, Visible::No, None, OsdLook::Hidden),
            (
                OsdPhase::Held {
                    look: OsdLook::Entering,
                    until: Stamp(10),
                },
                Visible::Yes,
                Some(10),
                OsdLook::Entering,
            ),
            (
                OsdPhase::Leaving { until: Stamp(20) },
                Visible::Yes,
                Some(20),
                OsdLook::Leaving,
            ),
        ];
        for &(phase, visible, wake, look) in CASES {
            assert_eq!(phase.visible(), visible, "{phase:?}");
            assert_eq!(phase.wake(), wake.map(Stamp), "{phase:?}");
            assert_eq!(phase.look(), look, "{phase:?}");
        }
    }

    #[test]
    fn a_run_of_key_presses_holds_from_the_last() {
        let presses = [0, 200, 400, 600];
        let phase = presses.iter().fold(OsdPhase::Hidden, |phase, &at| {
            step(phase, changed(at), TIMES)
        });
        assert_eq!(phase, held(OsdLook::Entering, 2100));
        let phase = step(phase, tick(2100), TIMES);
        let phase = step(phase, tick(2354), TIMES);
        assert_eq!(phase, OsdPhase::Hidden);
    }
}
