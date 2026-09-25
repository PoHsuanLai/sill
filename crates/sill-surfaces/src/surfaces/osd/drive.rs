//! The effects end of one indicator: follow the [`OsdLink`], feed the [`step`] machine, wake it
//! with quire's one timer (`ds::sleep`, for durations from `osd.hold_ms` and quire's motion
//! tokens, never a literal), map and unmap the surface, and move it when `osd.position` or the
//! gap changes. Once hidden, no task runs and nothing asks for a frame.

use std::time::{Duration, Instant};

use dioxus::core::Task;
use dioxus::prelude::*;
use shell_host::{OutputId, SurfaceHandle, SurfaceOp, Visibility, use_watch};
use sill_settings::OsdSettings;

use crate::FrameClock;
use crate::surfaces::launcher::presence::Visible;
use crate::surfaces::osd::layer::{anchor, margin};
use crate::surfaces::osd::link::{OsdChange, OsdLatest, OsdLink};
use crate::surfaces::osd::machine::{OsdInput, OsdPhase, OsdTimes, step};

/// The machine's times at motion `level` with `osd`'s hold.
pub fn times(osd: &OsdSettings, level: ds::MotionLevel) -> OsdTimes {
    let leave = ds::DurationToken::Move.duration(level) + ds::FRAME_SLACK;
    OsdTimes {
        hold_ms: u64::from(osd.hold_ms.0),
        leave_ms: u64::try_from(leave.as_millis()).unwrap_or(u64::MAX),
    }
}

/// What the view draws: the phase and the change it shows (the last one for this output).
#[derive(Debug, Clone, PartialEq)]
pub struct Drawn {
    pub phase: OsdPhase,
    pub change: Option<OsdChange>,
}

/// Run `output`'s indicator with `times`; returns what to draw.
pub fn use_osd(link: &OsdLink, output: OutputId, now_times: OsdTimes) -> Drawn {
    let latest = use_watch(link.subscribe());
    let clock = use_hook(|| FrameClock::new(Instant::now()));
    // A surface respawned after a close does not replay the change before it.
    let mut seen = use_hook(|| CopyValue::new(link.now().serial()));
    let mut shown = use_signal(|| None::<OsdChange>);
    let mut phase = use_signal(OsdPhase::default);
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
    use_follow_phase(phase.into());
    Drawn {
        phase: phase(),
        change: shown(),
    }
}

/// Wake the machine at each [`OsdPhase::wake`] until it is hidden.
async fn wake_loop(mut phase: Signal<OsdPhase>, clock: FrameClock, times: CopyValue<OsdTimes>) {
    loop {
        let wake = phase.peek().wake();
        let Some(at) = wake else { break };
        ds::sleep(Duration::from_millis(at.since(clock.now()))).await;
        let next = step(*phase.peek(), OsdInput::Tick(clock.now()), *times.peek());
        phase.set(next);
    }
}

/// Map the surface while the machine is up, unmap it once hidden. Nothing to do without a host
/// (a test, an SSR render).
fn use_follow_phase(phase: ReadSignal<OsdPhase>) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(None::<Visible>));
    use_effect(move || {
        let visible = phase.read().visible();
        if *applied.peek() == Some(visible) {
            return;
        }
        applied.set(Some(visible));
        if let Some(surface) = &surface {
            surface.apply(SurfaceOp::SetVisibility(match visible {
                Visible::Yes => Visibility::Shown,
                Visible::No => Visibility::Hidden,
            }));
        }
    });
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
        surface.apply(SurfaceOp::SetAnchor(anchor(osd.position)));
        surface.apply(SurfaceOp::SetMargin(margin(osd)));
    }
}

#[cfg(test)]
mod tests {
    use super::times;
    use ds::MotionLevel;
    use ds_settings::Ms;
    use sill_settings::OsdSettings;

    #[test]
    fn the_hold_is_the_setting_and_the_fade_is_quires_move() {
        let osd = OsdSettings {
            hold_ms: Ms(900),
            ..OsdSettings::default()
        };
        let standard = times(&osd, MotionLevel::Standard);
        assert_eq!(standard.hold_ms, 900);
        let want = ds::DurationToken::Move.duration(MotionLevel::Standard) + ds::FRAME_SLACK;
        assert_eq!(u128::from(standard.leave_ms), want.as_millis());
        let reduced = times(&osd, MotionLevel::Reduced);
        assert!(
            reduced.leave_ms < standard.leave_ms,
            "Reduced shortens the fade"
        );
    }
}
