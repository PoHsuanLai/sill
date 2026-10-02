//! One indicator's show and hold (design/20 §1.7): a change
//! shows it and restarts `osd.hold_ms`; when the hold ends it is hidden. Quire's `Osd` card owns
//! its own entrance and exit fade from that `shown` and calls `on_hidden` once the exit has
//! settled, so the surface unmaps then, not when this machine's hold ends. Pure: time is a
//! [`Stamp`] argument, and the surface wakes the machine at [`OsdHold::wake`].

use ds::base::time::stamp::Stamp;
use shell_host::Visibility;

/// How long the indicator holds after the last change, in milliseconds (`osd.hold_ms`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OsdTimes {
    pub hold_ms: u64,
}

/// Where one indicator is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OsdHold {
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
    /// The surface woke at this time (see [`OsdHold::wake`]).
    Tick(Stamp),
}

impl OsdHold {
    /// What `shown` the card gets.
    pub fn visible(self) -> Visibility {
        match self {
            OsdHold::Hidden => Visibility::Hidden,
            OsdHold::Held { .. } => Visibility::Shown,
        }
    }

    /// When the surface must wake the machine next; none once hidden, so an idle indicator
    /// runs no timer at all.
    pub fn wake(self) -> Option<Stamp> {
        match self {
            OsdHold::Hidden => None,
            OsdHold::Held { until } => Some(until),
        }
    }
}

/// `phase` after `input`.
pub fn step(phase: OsdHold, input: OsdInput, times: OsdTimes) -> OsdHold {
    match (phase, input) {
        (_, OsdInput::Changed(now)) => OsdHold::Held {
            until: now.after(times.hold_ms),
        },
        (OsdHold::Held { until }, OsdInput::Tick(now)) if now >= until => OsdHold::Hidden,
        (phase, OsdInput::Tick(_)) => phase,
    }
}

#[cfg(test)]
mod tests {
    use super::{OsdHold, OsdInput, OsdTimes, step};
    use ds::base::time::stamp::Stamp;
    use shell_host::Visibility;

    const TIMES: OsdTimes = OsdTimes { hold_ms: 1500 };

    fn held(until: u64) -> OsdHold {
        OsdHold::Held {
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
        let cases: &[(&str, OsdHold, OsdInput, OsdHold)] = &[
            (
                "a change shows it",
                OsdHold::Hidden,
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
                OsdHold::Hidden,
            ),
            (
                "a late wake still hides",
                held(1600),
                tick(1700),
                OsdHold::Hidden,
            ),
            (
                "a change while hidden shows it again",
                OsdHold::Hidden,
                changed(1700),
                held(3200),
            ),
            (
                "a wake while hidden changes nothing",
                OsdHold::Hidden,
                tick(5000),
                OsdHold::Hidden,
            ),
        ];
        for (name, before, input, want) in cases {
            assert_eq!(step(*before, *input, TIMES), *want, "{name}");
        }
    }

    #[test]
    fn only_a_held_indicator_is_visible_and_has_a_wake() {
        const CASES: &[(OsdHold, Visibility, Option<u64>)] = &[
            (OsdHold::Hidden, Visibility::Hidden, None),
            (
                OsdHold::Held { until: Stamp(10) },
                Visibility::Shown,
                Some(10),
            ),
        ];
        for &(phase, visible, wake) in CASES {
            assert_eq!(phase.visible(), visible, "{phase:?}");
            assert_eq!(phase.wake(), wake.map(Stamp), "{phase:?}");
        }
    }

    #[test]
    fn a_run_of_key_presses_holds_from_the_last() {
        let presses = [0, 200, 400, 600];
        let phase = presses.iter().fold(OsdHold::Hidden, |phase, &at| {
            step(phase, changed(at), TIMES)
        });
        assert_eq!(phase, held(2100));
        let phase = step(phase, tick(2100), TIMES);
        assert_eq!(phase, OsdHold::Hidden);
    }
}
