//! The runtime half of hiding the menu bar: the reveal machine in a signal, its frame drive and
//! one-shot timer, the compositor's edge reveals and the bar's open popups feeding it. The only
//! effectful part; every decision is `reveal::step`.
//!
//! Frames follow shell-host's frame hook as the dock's do: `RequestFrames` takes a `FrameDrive`,
//! every frame tick of the surface feeds `Frame`, and `StopFrames` drops the drive, so an idle
//! bar renders nothing. A `Redraw` stamps the time the next render draws the slide at.

use dioxus::prelude::*;
use ds::base::time::clock::sleep;
use ds::base::time::stamp::{FrameClock, Stamp};
use shell_host::{FrameDrive, OutputId, SurfaceHandle};
use sill_model::desktop::Shown;
use sill_settings::MenuBarHiding;
use sill_ui_kit::autohide::{Phase, Timing};
use sill_ui_kit::hooks::use_edge_reveal;
use sill_ui_kit::links::RevealEdge;

use super::io::MenuBar;
use super::reveal::{BarReveal, RevealEffect, RevealEvent, rest_phase, step};

/// The hiding as the bar's render sees it.
#[derive(Clone, Copy)]
pub(super) struct Hide {
    /// The machine: its phase says where the bar is.
    pub(super) machine: Signal<BarReveal>,
    /// The instant the current frame is drawn at.
    pub(super) drawn: Signal<Stamp>,
    feed: Callback<RevealEvent>,
}

impl Hide {
    /// Feed the machine.
    pub(super) fn feed(&self, event: RevealEvent) {
        self.feed.call(event);
    }
}

/// What the machine reads each render.
#[derive(Clone, Copy)]
struct Deps {
    timing: Timing,
    menus: MenuBar,
}

/// The mutable halves, all `Copy` handles.
#[derive(Clone, Copy)]
struct Slots {
    machine: Signal<BarReveal>,
    drawn: Signal<Stamp>,
    drive: CopyValue<Option<FrameDrive>>,
    deps: CopyValue<Deps>,
    /// The session's own feed, for its timer (set once the callback exists).
    feed: CopyValue<Option<Callback<RevealEvent>>>,
}

/// Start (once) and update (every render) the bar's hiding under `mode`, on `output`.
pub(super) fn use_hide(
    mode: MenuBarHiding,
    timing: Timing,
    menus: MenuBar,
    output: OutputId,
) -> Hide {
    let clock =
        use_hook(|| try_consume_context::<FrameClock>().unwrap_or_else(FrameClock::started));
    let surface = try_use_context::<SurfaceHandle>();
    let slots = Slots {
        machine: use_signal(|| BarReveal::at_rest(mode)),
        drawn: use_signal(|| Stamp(0)),
        drive: use_hook(|| CopyValue::new(None)),
        deps: use_hook(|| CopyValue::new(Deps { timing, menus })),
        feed: use_hook(|| CopyValue::new(None)),
    };
    let mut deps = slots.deps;
    deps.set(Deps { timing, menus });
    let feed = use_hook({
        let surface = surface.clone();
        move || {
            Callback::new(move |event: RevealEvent| handle(slots, surface.as_ref(), clock, event))
        }
    });
    let mut own = slots.feed;
    if own.peek().is_none() {
        own.set(Some(feed));
    }
    // A mode that changes what the bar rests as (hidden or fixed) starts the machine over; the
    // layer follows the same setting (`use_follow_layer`).
    let last = use_hook(|| CopyValue::new(mode));
    let mut machine = slots.machine;
    use_effect(use_reactive!(|mode| {
        let mut last = last;
        let before = last.replace(mode);
        if rest_phase(before) != rest_phase(mode) {
            machine.set(BarReveal::at_rest(mode));
        }
    }));
    // A popup opening shows a hidden bar (a toggle over IPC opens one with no pointer on it); the
    // last one closing lets it hide.
    let open = menus.any_open();
    use_effect(use_reactive!(|open| {
        feed.call(if open {
            RevealEvent::MenuOpened
        } else {
            RevealEvent::MenuClosed
        });
    }));
    use_edge_reveal(
        RevealEdge::Top,
        output,
        use_callback(move |shown: Shown| feed.call(RevealEvent::Edge(shown))),
    );
    use_hook(move || start_frame_task(feed, surface));
    Hide {
        machine: slots.machine,
        drawn: slots.drawn,
        feed,
    }
}

/// The session's frame task: each frame tick of the surface feeds `Frame`.
fn start_frame_task(feed: Callback<RevealEvent>, surface: Option<SurfaceHandle>) {
    if let Some(surface) = surface {
        spawn(async move {
            loop {
                let _tick = surface.next_frame().await;
                feed.call(RevealEvent::Frame);
            }
        });
    }
}

/// Step the machine with `event` at the clock's now and carry out the effects. A fixed bar has
/// nothing to track.
fn handle(slots: Slots, surface: Option<&SurfaceHandle>, clock: FrameClock, event: RevealEvent) {
    let mut machine = slots.machine;
    if machine.peek().phase == Phase::Fixed {
        return;
    }
    let now = clock.now();
    let deps = *slots.deps.peek();
    let (next, effects) = step(*machine.peek(), event, now, deps.menus.hold(), &deps.timing);
    if *machine.peek() != next {
        machine.set(next);
    }
    for effect in effects {
        apply(slots, surface, clock, now, effect);
    }
}

fn apply(
    slots: Slots,
    surface: Option<&SurfaceHandle>,
    clock: FrameClock,
    now: Stamp,
    effect: RevealEffect,
) {
    let mut drive = slots.drive;
    let mut drawn = slots.drawn;
    match effect {
        RevealEffect::Redraw => drawn.set(now),
        RevealEffect::RequestFrames => {
            if drive.peek().is_none()
                && let Some(surface) = surface
            {
                drive.set(Some(surface.drive_frames()));
            }
        }
        RevealEffect::StopFrames => {
            // The settle frame is drawn too; an idle tick (a colour transition) changes nothing.
            if drive.peek().is_some() {
                if *drawn.peek() != now {
                    drawn.set(now);
                }
                drive.set(None);
            }
        }
        RevealEffect::WakeAt(at) => wake(*slots.feed.peek(), clock, at),
        RevealEffect::SetInput(region) => {
            if let Some(surface) = surface {
                surface.set_input_region(region);
            }
        }
        RevealEffect::SetBlur(blur) => {
            if let Some(surface) = surface {
                surface.set_blur(blur);
            }
        }
    }
}

/// Feed `Tick` at `at` (the machine's one-shot timer).
fn wake(feed: Option<Callback<RevealEvent>>, clock: FrameClock, at: Stamp) {
    let Some(feed) = feed else { return };
    let wait = std::time::Duration::from_millis(at.since(clock.now()));
    spawn(async move {
        sleep(wait).await;
        feed.call(RevealEvent::Tick);
    });
}
