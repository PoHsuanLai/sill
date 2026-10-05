//! One opening's machine: quire's switcher ([`ds_behaviour::switcher::Switcher`], the one the
//! compositor runs too) with the one fact only this surface knows. Whether the show delay's wake
//! ends a tap depends on what the modifiers have told the opening (`hold.rs`), so the surface
//! provides that beside the app list as the machine's context at every step, including the step
//! the delay's wake causes.

use ds::base::machine::Machine;
use ds::base::time::stamp::Stamp;
use ds_behaviour::switcher::{Phase, SwIn, SwOut, Switcher, SwitcherParams};
use sill_launcher::AppId;

use super::hold::{Hold, tick_input};

/// The switcher over the apps' ids.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Opening(Switcher<AppId>);

impl Opening {
    /// An opening in `phase`.
    pub fn in_phase(phase: Phase) -> Opening {
        Opening(Switcher::in_phase(phase))
    }

    /// Where it is.
    pub fn phase(&self) -> Phase {
        self.0.phase()
    }
}

/// What a step reads from outside: the MRU apps, the current one first, and what the modifiers
/// have said.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Facts {
    pub apps: Vec<AppId>,
    pub hold: Hold,
}

impl Machine for Opening {
    type In = SwIn;
    type Out = SwOut<AppId>;
    type Params = SwitcherParams;
    type Ctx = Facts;

    fn step(
        self,
        input: SwIn,
        at: Stamp,
        params: &SwitcherParams,
        facts: &Facts,
    ) -> (Opening, Vec<SwOut<AppId>>) {
        let input = match input {
            SwIn::Elapsed => tick_input(self.phase(), facts.hold),
            other => other,
        };
        let (next, outs) = self.0.step(input, at, params, &facts.apps);
        (Opening(next), outs)
    }

    fn wake(&self) -> Option<Stamp> {
        self.0.wake()
    }
}
