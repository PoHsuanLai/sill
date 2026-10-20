//! The app switcher's asks: the chord verbs from `sill switcher` and the made-up open from
//! `sill debug switcher`, written to the switcher's link for the output it opens on.
//!
//! Which output: [`Outputs::open_on`](crate::outputs::Outputs::open_on)'s rule.

use sill_ipc::act::SwitcherRequest;
use sill_ipc::response::Response;
use sill_overlays::Switcher;
use sill_ui_kit::links::{Dir, SwitcherAsk};

use super::reach::Reach;
use super::reply::unsupported;

/// `sill switcher next|prev|cancel|commit`.
pub fn chord(reach: &Reach, request: SwitcherRequest) -> Response {
    ask(reach, ask_of(request))
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

fn ask_of(request: SwitcherRequest) -> SwitcherAsk {
    match request {
        SwitcherRequest::Next => SwitcherAsk::Chord(Dir::Next),
        SwitcherRequest::Prev => SwitcherAsk::Chord(Dir::Prev),
        SwitcherRequest::Cancel => SwitcherAsk::Cancel,
        SwitcherRequest::Commit => SwitcherAsk::Commit,
    }
}

#[cfg(test)]
mod tests {
    use sill_ipc::act::SwitcherRequest;
    use sill_ui_kit::links::{Dir, SwitcherAsk};

    use super::ask_of;

    #[test]
    fn requests_become_asks() {
        let cases = [
            (SwitcherRequest::Next, SwitcherAsk::Chord(Dir::Next)),
            (SwitcherRequest::Prev, SwitcherAsk::Chord(Dir::Prev)),
            (SwitcherRequest::Cancel, SwitcherAsk::Cancel),
            (SwitcherRequest::Commit, SwitcherAsk::Commit),
        ];
        for (request, want) in cases {
            assert_eq!(ask_of(request), want, "{request:?}");
        }
    }
}
