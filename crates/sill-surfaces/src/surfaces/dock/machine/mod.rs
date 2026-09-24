//! The dock's state machine (design/10 §10.4): pure, `now` an argument, effects returned. The
//! component hit-tests pointer events against the frame's layout and feeds them here; it
//! applies the effects and draws [`displayed`] with `sill_services::magnify`.
//!
//! Differences from the doc's sketch, all recorded in FINDINGS.md: pointer events carry the
//! component's hit test (`Hit`, `DropZone`) instead of bare coordinates, so the machine needs no
//! layout; release carries the tile's app state so the click's action (10.3.6) is decided here;
//! `Tick` and `WakeAt` are the one-shot timer the doc mentions in prose; `Reveal::Fixed` is the
//! auto-hide-off state; geometry is `Cpx` and progress `Permille`, never floats.

mod bounce;
mod hover;
mod label;
mod params;
mod press;
mod reveal;
mod types;

pub use bounce::{Bounce, BounceKind, lift};
pub use hover::{FreezeCause, Hover, Moved, displayed};
pub use label::Label;
pub use params::DockParams;
pub use press::{ActivateMode, DropZone, Instance, Press};
pub use reveal::{Reveal, exclusive_zone, slide_offset};
pub(crate) use reveal::{hit_region, pill_region, strip_region};
pub use types::{
    AppChange, AttentionKind, Button, DockEffect, DockEvent, DockKey, DockState, Hit, MenuState,
    Modifier, Point, Pointer, Spring, TileApp, TileId,
};

use sill_services::Stamp;

/// `dock` after `event` at `now`, and what the component must do about it.
pub fn step(
    dock: DockState,
    event: DockEvent,
    now: Stamp,
    p: &DockParams,
) -> (DockState, Vec<DockEffect>) {
    let mut out = Vec::new();
    let dock = match reveal::gate(&dock, &event) {
        reveal::Gate::Reveal => reveal::step(dock, &event, now, p, &mut out),
        reveal::Gate::Dock => {
            let dock = reveal::step(dock, &event, now, p, &mut out);
            let dock = press::step(dock, &event, now, p, &mut out);
            let dock = label::step(dock, &event, now, p, &mut out);
            let dock = hover::step(dock, &event, now, p, &mut out);
            bounce::step(dock, &event, now, p, &mut out)
        }
    };
    let dock = frames(dock, &event, now, p, &mut out);
    (dock, out)
}

/// On a frame: redraw while anything moves, stop frames once nothing does.
fn frames(
    dock: DockState,
    event: &DockEvent,
    now: Stamp,
    p: &DockParams,
    out: &mut Vec<DockEffect>,
) -> DockState {
    if !matches!(event, DockEvent::Frame) {
        return dock;
    }
    let moving = hover::animating(&dock.hover)
        || !dock.bounces.is_empty()
        || reveal::animating(&dock.reveal)
        || matches!(dock.press, Press::Dragging { .. });
    let pending = matches!(
        dock.hover,
        Hover::Tracking {
            moved: Moved::Pending
        }
    );
    let dock = hover::drawn(dock);
    if moving || pending {
        out.push(DockEffect::Redraw);
    } else {
        out.push(DockEffect::StopFrames);
    }
    let _ = (now, p);
    dock
}

#[cfg(test)]
mod settle_tests;
