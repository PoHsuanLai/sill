//! The effects end of the trial sheet: follow the [`DisplayTrialLink`] and the banners' output
//! onto the stage (which maps the surface), write the person's answer to the link, and count the
//! seconds down on quire's one timer.

use std::time::Duration;

use dioxus::prelude::*;
use ds::base::time::clock::sleep;
use ds::base::time::stamp::FrameClock;
use shell_host::{OutputInfo, use_watch};
use sill_model::desktop::{TrialAnswer, TrialId, seconds_left, until_next_second};
use sill_ui_kit::links::{DisplayTrialLink, TrialRun};
use sill_ui_kit::sheet_stage::{SheetStage, use_sheet_stage};

/// The stage on `output`, following the open trial and the banners' output.
pub(super) fn use_stage(output: &OutputInfo) -> Signal<SheetStage<TrialRun>> {
    let link = use_context::<DisplayTrialLink>();
    let open = use_watch(link.subscribe());
    let shows = use_watch(link.banners().subscribe());
    let id = output.id;
    use_sheet_stage(super::surface::PRESET, move || {
        let current = *open.read();
        shows.read().shows_on(id).then_some(current).flatten()
    })
}

/// Answers trial `trial` with the answer it is called with.
pub(super) fn use_answer(trial: TrialId) -> Callback<TrialAnswer> {
    let link = use_context::<DisplayTrialLink>();
    use_callback(move |answer: TrialAnswer| link.answer(trial, answer))
}

/// The whole seconds left of `seconds`, counted from the moment this is first called and
/// changed on each second. The card is keyed by the send of the trial, so a restart counts again
/// from its own start.
pub(super) fn use_left(seconds: u32) -> ReadSignal<u32> {
    let clock = use_hook(FrameClock::started);
    let mut left = use_signal(|| seconds);
    use_future(move || async move {
        loop {
            let elapsed = clock.now().0;
            let now_left = seconds_left(seconds, elapsed);
            if *left.peek() != now_left {
                left.set(now_left);
            }
            if now_left == 0 {
                break;
            }
            sleep(Duration::from_millis(until_next_second(elapsed))).await;
        }
    });
    left.into()
}
