//! The effects end of one indicator: follow the [`OsdLink`], feed the [`step`] machine, wake it
//! with quire's one timer (`ds::base::time::clock::sleep`, for durations from [`HOLD`], never a literal), map
//! the surface once the machine shows something, unmap it once quire's `Osd` card says its exit
//! has settled (`on_hidden`). Once hidden and
//! gone, no task runs and nothing asks for a frame.

use ds::base::time::clock::sleep;
use std::time::Duration;

use dioxus::core::Task;
use dioxus::prelude::*;
use shell_host::{OutputId, use_watch};

use super::step::{OsdHold, OsdInput, OsdTimes, step};
use ds::base::time::stamp::FrameClock;
use sill_ui_kit::hooks::{use_map_when_shown, use_unmap_on_hidden};
use sill_ui_kit::links::{OsdChange, OsdLink};

/// How long the indicator holds after the last change.
const HOLD: ds::base::time::stamp::Ms = ds::base::time::stamp::Ms(1500);

/// The machine's times.
pub fn times() -> OsdTimes {
    OsdTimes { hold_ms: HOLD }
}

/// What the view draws: the phase, the change it shows (the last one for this output), and the
/// handler for quire's `Osd::on_hidden`.
#[derive(Clone)]
pub(crate) struct Drawn {
    pub phase: OsdHold,
    pub change: Option<OsdChange>,
    pub on_hidden: EventHandler<()>,
}

/// Run `output`'s indicator with `times`; returns what to draw.
pub(crate) fn use_osd(link: &OsdLink, output: OutputId, now_times: OsdTimes) -> Drawn {
    let latest = use_watch(link.subscribe());
    let clock = use_hook(FrameClock::started);
    // A surface respawned after a close does not replay the change before it.
    let mut seen = use_hook(|| CopyValue::new(link.now().map(|c| c.serial).unwrap_or_default()));
    let mut shown = use_signal(|| None::<OsdChange>);
    let mut phase = use_signal(OsdHold::default);
    let mut waking = use_hook(|| CopyValue::new(None::<Task>));
    // Read by the handlers at each change and wake; a settings change applies from the next.
    let mut times = use_hook(|| CopyValue::new(now_times));
    if *times.peek() != now_times {
        times.set(now_times);
    }
    use_effect(move || {
        let fresh = latest
            .read()
            .clone()
            .filter(|change| change.is_fresh_for(*seen.peek(), output));
        let Some(change) = fresh else {
            return;
        };
        seen.set(change.serial);
        shown.set(Some(change));
        let next = step(*phase.peek(), OsdInput::Changed(clock.now()), *times.peek());
        phase.set(next);
        if let Some(task) = waking.take() {
            task.cancel();
        }
        waking.set(Some(spawn(wake_loop(phase, clock, times))));
    });
    use_map_when_shown(move || phase.read().visible());
    Drawn {
        phase: phase(),
        change: shown(),
        on_hidden: use_unmap_on_hidden(),
    }
}

/// Wake the machine at each [`OsdHold::wake`] until it is hidden.
async fn wake_loop(mut phase: Signal<OsdHold>, clock: FrameClock, times: CopyValue<OsdTimes>) {
    loop {
        let wake = phase.peek().wake();
        let Some(at) = wake else { break };
        sleep(Duration::from_millis(at.since(clock.now()))).await;
        let next = step(*phase.peek(), OsdInput::Tick(clock.now()), *times.peek());
        phase.set(next);
    }
}

#[cfg(test)]
mod tests {
    use super::times;

    #[test]
    fn the_hold_is_one_and_a_half_seconds() {
        assert_eq!(times().hold_ms, ds::base::time::stamp::Ms(1500));
    }
}
