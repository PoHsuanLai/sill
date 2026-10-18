//! The effects end of one indicator: follow the [`OsdLink`], feed the [`step`] machine, wake it
//! with quire's one timer (`ds::base::time::clock::sleep`, for durations from `osd.hold_ms`, never a literal), map
//! the surface once the machine shows something, unmap it once quire's `Osd` card says its exit
//! has settled (`on_hidden`), and move it when `osd.position` or the gap changes. Once hidden and
//! gone, no task runs and nothing asks for a frame.

use ds::base::time::clock::sleep;
use std::time::Duration;

use dioxus::core::Task;
use dioxus::prelude::*;
use shell_host::{OutputId, SurfaceHandle, Visibility, use_watch};
use sill_settings::OsdSettings;

use super::layer::{anchor, margin};
use super::step::{OsdHold, OsdInput, OsdTimes, step};
use ds::base::time::stamp::FrameClock;
use sill_ui_kit::links::{OsdChange, OsdLatest, OsdLink};

/// The machine's times from `osd`'s hold.
pub fn times(osd: &OsdSettings) -> OsdTimes {
    OsdTimes {
        hold_ms: ds::base::time::stamp::Ms::from(osd.hold_ms.0),
    }
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
    let mut seen = use_hook(|| CopyValue::new(link.now().serial()));
    let mut shown = use_signal(|| None::<OsdChange>);
    let mut phase = use_signal(OsdHold::default);
    let mut waking = use_hook(|| CopyValue::new(None::<Task>));
    // Read by the handlers at each change and wake; a settings change applies from the next.
    let mut times = use_hook(|| CopyValue::new(now_times));
    if *times.peek() != now_times {
        times.set(now_times);
    }
    use_effect(move || {
        let latest: OsdLatest = latest.read().clone();
        let Some(change) = latest.fresh_for(*seen.peek(), output).cloned() else {
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
    use_map_when_shown(phase.into());
    Drawn {
        phase: phase(),
        change: shown(),
        on_hidden: use_unmap(),
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

/// Map the surface once the machine shows something. Never unmaps here: quire's `Osd` card owns
/// the exit fade, and [`use_unmap`] is the surface's only way down, once it has settled. Nothing
/// to do without a host (a test, an SSR render).
fn use_map_when_shown(phase: ReadSignal<OsdHold>) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(Visibility::Hidden));
    use_effect(move || {
        let visible = phase.read().visible();
        if *applied.peek() == visible {
            return;
        }
        applied.set(visible);
        if visible == Visibility::Shown
            && let Some(surface) = &surface
        {
            surface.set_visibility(Visibility::Shown);
        }
    });
}

/// The handler for quire's `Osd::on_hidden`: its exit has settled, so unmap the surface. Nothing
/// to do without a host (a test, an SSR render).
fn use_unmap() -> EventHandler<()> {
    let surface = try_use_context::<SurfaceHandle>();
    use_hook(|| {
        EventHandler::new(move |()| {
            if let Some(surface) = &surface {
                surface.set_visibility(Visibility::Hidden);
            }
        })
    })
}

/// Move the surface when `osd.position` or `osd.margin_px` changes (the size is fixed).
pub fn use_follow_position(osd: &OsdSettings) {
    let surface = try_use_context::<SurfaceHandle>();
    let wanted = (osd.position, osd.margin_px);
    let mut applied = use_hook(|| CopyValue::new(wanted));
    if *applied.peek() == wanted {
        return;
    }
    applied.set(wanted);
    if let Some(surface) = &surface {
        surface.set_anchor(anchor(osd.position));
        surface.set_margin(margin(osd));
    }
}

#[cfg(test)]
mod tests {
    use super::times;
    use ds_settings::Ms;
    use sill_settings::OsdSettings;

    #[test]
    fn the_hold_is_the_setting() {
        let osd = OsdSettings {
            hold_ms: Ms(900),
            ..OsdSettings::default()
        };
        assert_eq!(times(&osd).hold_ms, ds::base::time::stamp::Ms(900));
    }
}
