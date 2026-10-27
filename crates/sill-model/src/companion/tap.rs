//! The double-tap machine: a lone tap of the key arms it, a second tap inside the window
//! summons. It is the reference for the compositor's own detector (route B of the keyboard
//! remapper sends one tap each time; route A and the compositor send a summon outright).
//!
//! The deadline lives in the state (`Armed { until }`), so a machine at rest wakes nothing: an
//! idle companion runs no timer.

use ds_core::machine::{Elapsed, Machine};
use ds_core::time::stamp::Stamp;

use crate::units::Ms;

/// Where the double tap stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tap {
    /// No tap pending.
    #[default]
    Rest,
    /// One tap was seen; a second before `until` summons.
    Armed {
        /// The last instant a second tap counts.
        until: Stamp,
    },
}

/// What moves the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TapIn {
    /// The key was tapped.
    Tap,
    /// The window passed.
    Elapsed,
}

impl From<Elapsed> for TapIn {
    fn from(_: Elapsed) -> TapIn {
        TapIn::Elapsed
    }
}

/// What the machine asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TapOut {
    /// Summon the companion.
    Summon,
}

/// How long a second tap may follow the first .
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TapParams {
    /// The window.
    pub window: Ms,
}

impl Default for TapParams {
    fn default() -> TapParams {
        TapParams { window: Ms(350) }
    }
}

impl Machine for Tap {
    type In = TapIn;
    type Out = TapOut;
    type Params = TapParams;
    type Ctx = ();

    fn step(self, input: TapIn, at: Stamp, params: &TapParams, _: &()) -> (Tap, Vec<TapOut>) {
        match (self, input) {
            (Tap::Rest, TapIn::Tap) => (
                Tap::Armed {
                    until: at.after(params.window.span()),
                },
                Vec::new(),
            ),
            (Tap::Armed { until }, TapIn::Tap) if at <= until => (Tap::Rest, vec![TapOut::Summon]),
            // A tap after the window is the first of a new pair.
            (Tap::Armed { .. }, TapIn::Tap) => (
                Tap::Armed {
                    until: at.after(params.window.span()),
                },
                Vec::new(),
            ),
            (Tap::Armed { until }, TapIn::Elapsed) if at >= until => (Tap::Rest, Vec::new()),
            (state, TapIn::Elapsed) => (state, Vec::new()),
        }
    }

    fn wake(&self) -> Option<Stamp> {
        match self {
            Tap::Rest => None,
            Tap::Armed { until } => Some(*until),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Case = (&'static str, Tap, TapIn, u64, Tap, &'static [TapOut]);

    #[test]
    fn tap_machine_table() {
        let armed = |t| Tap::Armed { until: Stamp(t) };
        #[rustfmt::skip]
        let cases: &[Case] = &[
            ("first tap arms",                     Tap::Rest,  TapIn::Tap,     1000, armed(1350), &[]),
            ("second tap inside the window",       armed(1350), TapIn::Tap,    1200, Tap::Rest,   &[TapOut::Summon]),
            ("second tap on the last instant",     armed(1350), TapIn::Tap,    1350, Tap::Rest,   &[TapOut::Summon]),
            ("a tap after the window starts over", armed(1350), TapIn::Tap,    1351, armed(1701), &[]),
            ("elapsed at the deadline rests",      armed(1350), TapIn::Elapsed, 1350, Tap::Rest,  &[]),
            ("early elapsed changes nothing",      armed(1350), TapIn::Elapsed, 1349, armed(1350), &[]),
            ("elapsed at rest is nothing",         Tap::Rest,  TapIn::Elapsed, 50,   Tap::Rest,   &[]),
        ];
        for (name, from, input, at, next, outs) in cases {
            let (state, out) = from.step(*input, Stamp(*at), &TapParams::default(), &());
            assert_eq!(state, *next, "{name}: state");
            assert_eq!(out.as_slice(), *outs, "{name}: outputs");
        }
    }

    #[test]
    fn a_triple_tap_is_one_summon_and_an_armed_state() {
        let params = TapParams::default();
        let (state, first) = Tap::Rest.step(TapIn::Tap, Stamp(0), &params, &());
        let (state, second) = state.step(TapIn::Tap, Stamp(100), &params, &());
        let (state, third) = state.step(TapIn::Tap, Stamp(200), &params, &());
        assert_eq!(
            (first, second, third),
            (Vec::new(), vec![TapOut::Summon], Vec::new())
        );
        assert_eq!(state, Tap::Armed { until: Stamp(550) });
    }

    #[test]
    fn tap_rest_has_no_wake() {
        assert_eq!(Tap::Rest.wake(), None);
        assert_eq!(Tap::Armed { until: Stamp(9) }.wake(), Some(Stamp(9)));
    }
}
