//! The app switcher's asks: the chord verbs from `sill switcher` and the made-up open from
//! `sill debug switcher`, written to the switcher's link for the output it opens on.
//!
//! Which output: the one under the pointer (design/13) is not known to the daemon, so the main
//! display (the banners' output).

use shell_host::OutputId;
use sill_ipc::act::SwitcherRequest;
use sill_ipc::response::Response;
use sill_ui_kit::Switcher;
use sill_ui_kit::links::BannerOutput;
use sill_ui_kit::links::{Dir, SwitcherAsk};

use super::reach::Reach;
use super::reply::unsupported;
use crate::outputs::Outputs;

/// `sill switcher next|prev|cancel|commit`.
pub fn chord(reach: &Reach, request: SwitcherRequest) -> Response {
    ask(reach, ask_of(request))
}

/// Write `ask` for the switcher's output.
pub fn ask(reach: &Reach, ask: SwitcherAsk) -> Response {
    match target(reach.outputs, reach.links.banners.now()) {
        Some(output) => {
            reach.links.switcher.ask(output, ask);
            Response::Done
        }
        None => unsupported("no output has a switcher surface"),
    }
}

/// The output the switcher opens on: the banners' (the main display), else the first with a
/// switcher surface.
fn target(outputs: &Outputs, banners: BannerOutput) -> Option<OutputId> {
    let has_switcher = |id: OutputId| {
        outputs
            .get(id)
            .is_some_and(|s| s.surface_of::<Switcher>().is_some())
    };
    match banners {
        BannerOutput::On(id) if has_switcher(id) => Some(id),
        _ => outputs
            .outputs
            .iter()
            .find(|(_, s)| s.surface_of::<Switcher>().is_some())
            .map(|(info, _)| info.id),
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
