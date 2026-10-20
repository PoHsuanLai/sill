//! The app switcher's asks: the switcher actions and the made-up open from `sill debug switcher`,
//! written to the switcher's link for the output it opens on.
//!
//! Which output: [`Outputs::open_on`](crate::outputs::Outputs::open_on)'s rule.

use sill_ipc::response::Response;
use sill_model::actions::SwitcherStep;
use sill_overlays::Switcher;
use sill_ui_kit::links::{Dir, SwitcherAsk};

use crate::ipc::dispatch::{Reach, unsupported};

/// A step of the switcher.
pub fn chord(reach: &Reach, step: SwitcherStep) -> Response {
    ask(reach, ask_of(step))
}

/// Write `ask` for the switcher's output.
pub fn ask(reach: &Reach, ask: SwitcherAsk) -> Response {
    match reach.outputs.open_on::<Switcher>(reach.services) {
        Some(output) => {
            reach.links.switcher.ask(output, ask);
            Response::Done
        }
        None => unsupported("no output has a switcher surface"),
    }
}

fn ask_of(step: SwitcherStep) -> SwitcherAsk {
    match step {
        SwitcherStep::Next => SwitcherAsk::Chord(Dir::Next),
        SwitcherStep::Prev => SwitcherAsk::Chord(Dir::Prev),
        SwitcherStep::Cancel => SwitcherAsk::Cancel,
        SwitcherStep::Commit => SwitcherAsk::Commit,
    }
}

#[cfg(test)]
mod tests {
    use sill_model::actions::SwitcherStep;
    use sill_ui_kit::links::{Dir, SwitcherAsk};

    use super::ask_of;

    #[test]
    fn steps_become_asks() {
        let cases = [
            (SwitcherStep::Next, SwitcherAsk::Chord(Dir::Next)),
            (SwitcherStep::Prev, SwitcherAsk::Chord(Dir::Prev)),
            (SwitcherStep::Cancel, SwitcherAsk::Cancel),
            (SwitcherStep::Commit, SwitcherAsk::Commit),
        ];
        for (step, want) in cases {
            assert_eq!(ask_of(step), want, "{step:?}");
        }
    }
}
