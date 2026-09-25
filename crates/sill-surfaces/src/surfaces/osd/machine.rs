//! One indicator's show and hold (design/20 §1.7; quire CONSUMING.md "OSD parts"): a change
//! shows it and restarts `osd.hold_ms`; when the hold ends it is hidden. Quire's `Osd` card owns
//! its own entrance and exit fade from that `shown` and calls `on_hidden` once the exit has
//! settled, so the surface unmaps then, not when this machine's hold ends. Pure: time is a
//! [`Stamp`] argument, and the surface wakes the machine at [`OsdPhase::wake`].

use sill_services::Stamp;

use crate::surfaces::launcher::presence::Visible;

/// How long the indicator holds after the last change, in milliseconds (`osd.hold_ms`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OsdTimes {
    pub hold_ms: u64,
}

/// Where one indicator is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OsdPhase {
    /// `shown` is hidden: quire's card is fading out, or already gone.
    #[default]
    Hidden,
    /// `shown` is visible, until `until`.
    Held { until: Stamp },
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
    /// What `shown` the card gets.
    pub fn visible(self) -> Visible {
        match self {
            OsdPhase::Hidden => Visible::No,
            OsdPhase::Held { .. } => Visible::Yes,
        }
    }

    /// When the surface must wake the machine next; none once hidden, so an idle indicator
    /// runs no timer at all.
    pub fn wake(self) -> Option<Stamp> {
        match self {
            OsdPhase::Hidden => None,
            OsdPhase::Held { until } => Some(until),
        }
    }
}

/// `phase` after `input`.
pub fn step(phase: OsdPhase, input: OsdInput, times: OsdTimes) -> OsdPhase {
    match (phase, input) {
        (_, OsdInput::Changed(now)) => OsdPhase::Held {
            until: now.after(times.hold_ms),
        },
        (OsdPhase::Held { until }, OsdInput::Tick(now)) if now >= until => OsdPhase::Hidden,
        (phase, OsdInput::Tick(_)) => phase,
    }
}

#[cfg(test)]
mod tests {
    use super::{OsdInput, OsdPhase, OsdTimes, step};
    use crate::surfaces::launcher::presence::Visible;
    use sill_services::Stamp;

    const TIMES: OsdTimes = OsdTimes { hold_ms: 1500 };

    fn held(until: u64) -> OsdPhase {
        OsdPhase::Held {
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
    fn show_and_hold_follow_the_table() {
        let cases: &[(&str, OsdPhase, OsdInput, OsdPhase)] = &[
            (
                "a change shows it",
                OsdPhase::Hidden,
                changed(100),
                held(1600),
            ),
            (
                "another change restarts the hold",
                held(1600),
                changed(900),
                held(2400),
            ),
            (
                "an early wake changes nothing",
                held(1600),
                tick(1599),
                held(1600),
            ),
            (
                "the hold ends: hidden",
                held(1600),
                tick(1600),
                OsdPhase::Hidden,
            ),
            (
                "a late wake still hides",
                held(1600),
                tick(1700),
                OsdPhase::Hidden,
            ),
            (
                "a change while hidden shows it again",
                OsdPhase::Hidden,
                changed(1700),
                held(3200),
            ),
            (
                "a wake while hidden changes nothing",
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
    fn only_a_held_indicator_is_visible_and_has_a_wake() {
        const CASES: &[(OsdPhase, Visible, Option<u64>)] = &[
            (OsdPhase::Hidden, Visible::No, None),
            (OsdPhase::Held { until: Stamp(10) }, Visible::Yes, Some(10)),
        ];
        for &(phase, visible, wake) in CASES {
            assert_eq!(phase.visible(), visible, "{phase:?}");
            assert_eq!(phase.wake(), wake.map(Stamp), "{phase:?}");
        }
    }

    #[test]
    fn a_run_of_key_presses_holds_from_the_last() {
        let presses = [0, 200, 400, 600];
        let phase = presses.iter().fold(OsdPhase::Hidden, |phase, &at| {
            step(phase, changed(at), TIMES)
        });
        assert_eq!(phase, held(2100));
        let phase = step(phase, tick(2100), TIMES);
        assert_eq!(phase, OsdPhase::Hidden);
    }
}
