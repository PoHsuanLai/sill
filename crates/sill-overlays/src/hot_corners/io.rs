//! The effects end of one corner: feed the pointer's enter and leave to the trigger machine
//! (`ds_behaviour::hot_corner::step`), run its dwell and re-arm on quire's one timer (`ds::base::time::clock::sleep`), and write a firing
//! to the [`CornerLink`].

use ds::base::time::clock::sleep;
use std::time::Duration;

use dioxus::core::Task;
use dioxus::prelude::*;
use ds_behaviour::hot_corner::{CornerIn, CornerMachine, CornerOut, CornerParams, step};
use shell_host::OutputId;
use sill_ui_kit::hooks::use_settings;
use sill_ui_kit::links::{Corner, CornerLink};

/// One corner's machine, its running timer and where its firings go. Cheap to clone: signals,
/// cells and a link.
#[derive(Clone)]
pub(super) struct Pointer {
    machine: Signal<CornerMachine>,
    waking: CopyValue<Option<Task>>,
    params: CopyValue<CornerParams>,
    link: CornerLink,
    output: OutputId,
    corner: Corner,
}

/// Run `corner` of `output`: the machine on `hot_corners.*`, read at each step (a settings
/// change applies from the next).
pub(super) fn use_hot_corner(output: OutputId, corner: Corner) -> Pointer {
    let link = use_context::<CornerLink>();
    let file = use_settings()();
    let now_params = file.hot_corners.params();
    let machine = use_signal(CornerMachine::default);
    let waking = use_hook(|| CopyValue::new(None::<Task>));
    let mut params = use_hook(|| CopyValue::new(now_params));
    if *params.peek() != now_params {
        params.set(now_params);
    }
    Pointer {
        machine,
        waking,
        params,
        link,
        output,
        corner,
    }
}

impl Pointer {
    /// The pointer entered the corner.
    pub(super) fn enter(&self) {
        self.feed(CornerIn::Enter);
    }

    /// The pointer left the corner.
    pub(super) fn leave(&self) {
        self.feed(CornerIn::Leave);
    }

    /// Feed `input` to the machine, applying every effect: `Fire` hits the link,
    /// `StartDwell`/`StartRearm` (re)arm the timer and feed the elapsed event back in when it
    /// wakes (a stale token is ignored by `step`, so a timer that outlives its phase is
    /// harmless; cancelling it early, as here, just skips that wait).
    fn feed(&self, input: CornerIn) {
        let mut machine = self.machine;
        let (next, outs) = step(*machine.peek(), input, *self.params.peek());
        machine.set(next);
        for out in outs {
            match out {
                CornerOut::Fire => self.link.hit(self.corner, self.output),
                CornerOut::StartDwell(token, wait) => {
                    self.start(wait, CornerIn::DwellElapsed(token))
                }
                CornerOut::StartRearm(token, wait) => {
                    self.start(wait, CornerIn::RearmElapsed(token))
                }
            }
        }
    }

    /// Cancel the running timer (if any) and start a fresh one: sleep `wait`, then feed `then`.
    fn start(&self, wait: Duration, then: CornerIn) {
        let mut waking = self.waking;
        if let Some(old) = waking.take() {
            old.cancel();
        }
        let pointer = self.clone();
        waking.set(Some(spawn(async move {
            sleep(wait).await;
            pointer.feed(then);
        })));
    }
}
