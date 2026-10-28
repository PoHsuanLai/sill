//! Magnification progress (design/10 §10.3.3 table, §10.4 hover transitions, §10.5.4).
//! States hold the *linear* progress; [`displayed`] eases it for drawing.

use ds::base::time::stamp::Stamp;
use sill_model::units::{Cpx, Permille};
use sill_settings::OnOff;

use super::params::DockParams;
use super::params::eased;
use super::types::{DockEffect, DockEvent, DockState, Hit};

/// Why magnification is frozen at its last frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreezeCause {
    Menu,
    Drag,
    Slide,
}

/// The magnification's phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Hover {
    #[default]
    Rest,
    Entering {
        from: Permille,
        since: Stamp,
    },
    Tracking {
        /// Whether a pointer move since the last frame still needs drawing.
        pending: bool,
    },
    Leaving {
        from: Permille,
        since: Stamp,
    },
    /// `shown` is the eased value on screen, `linear` where a later leave starts from.
    Frozen {
        shown: Permille,
        linear: Permille,
        cause: FreezeCause,
    },
}

/// Linear progress at `now`.
pub(crate) fn linear(hover: &Hover, now: Stamp, p: &DockParams) -> Permille {
    match *hover {
        Hover::Rest => Permille::ZERO,
        Hover::Tracking { .. } => Permille::ONE,
        Hover::Frozen { linear, .. } => linear,
        Hover::Entering { from, since } => {
            let gained = per_mille(now.since(since), p.enter_ms);
            Permille((from.0 + gained).min(1000))
        }
        Hover::Leaving { from, since } => {
            let lost = u64::from(from.0)
                * u64::from(per_mille(now.since(since), p.leave_ms).min(1000))
                / 1000;
            Permille(
                from.0
                    .saturating_sub(u32::try_from(lost).unwrap_or(u32::MAX)),
            )
        }
    }
}

/// `elapsed / total` in thousandths, uncapped; a zero total is instantly complete.
fn per_mille(elapsed: u64, total: u64) -> u32 {
    if total == 0 {
        return 1000;
    }
    u32::try_from(elapsed * 1000 / total)
        .unwrap_or(u32::MAX)
        .min(1_000_000)
}

/// The eased progress and pointer x to draw at `now` (design/10 §10.5.4); zero progress when
/// magnification is off.
pub fn displayed(dock: &DockState, now: Stamp, p: &DockParams) -> (Permille, Cpx) {
    if p.magnification == OnOff::Off {
        return (Permille::ZERO, dock.x);
    }
    let lin = linear(&dock.hover, now, p);
    let shown = match dock.hover {
        Hover::Rest => Permille::ZERO,
        Hover::Frozen { shown, .. } => shown,
        Hover::Entering { .. } | Hover::Tracking { .. } => eased(p.ease_out, lin),
        Hover::Leaving { .. } => {
            Permille(1000u32.saturating_sub(eased(p.ease_out, Permille(1000 - lin.0.min(1000))).0))
        }
    };
    (shown, dock.x)
}

/// Whether the phase needs a frame every refresh.
pub(crate) fn animating(hover: &Hover) -> bool {
    matches!(hover, Hover::Entering { .. } | Hover::Leaving { .. })
}

/// A pending move marked drawn (after a frame's redraw).
pub(crate) fn drawn(dock: DockState) -> DockState {
    match dock.hover {
        Hover::Tracking { .. } => DockState {
            hover: Hover::Tracking { pending: false },
            ..dock
        },
        _ => dock,
    }
}

/// Freeze at the current frame (a menu opened, a drag left the dock, a slide began).
pub(crate) fn freeze(dock: DockState, cause: FreezeCause, now: Stamp, p: &DockParams) -> DockState {
    if matches!(dock.hover, Hover::Rest | Hover::Frozen { .. }) {
        return dock;
    }
    let (shown, _) = displayed(&dock, now, p);
    let linear = linear(&dock.hover, now, p);
    DockState {
        hover: Hover::Frozen {
            shown,
            linear,
            cause,
        },
        ..dock
    }
}

/// Resume after a freeze: tracking if the pointer is inside, else leave from where it stood.
pub(crate) fn thaw(dock: DockState, now: Stamp, out: &mut Vec<DockEffect>) -> DockState {
    let Hover::Frozen { linear, .. } = dock.hover else {
        return dock;
    };
    out.push(DockEffect::RequestFrames);
    let hover = if dock.pointer_inside {
        Hover::Tracking { pending: true }
    } else {
        Hover::Leaving {
            from: linear,
            since: now,
        }
    };
    DockState { hover, ..dock }
}

/// The hover half of [`super::step`].
pub(crate) fn step(
    dock: DockState,
    event: &DockEvent,
    now: Stamp,
    p: &DockParams,
    out: &mut Vec<DockEffect>,
) -> DockState {
    match *event {
        DockEvent::PointerEnter { at, hit } | DockEvent::PointerMove { at, hit, .. }
            if hit != Hit::Outside =>
        {
            moved(dock, at.x, now, p, out)
        }
        DockEvent::DndMotion { x, hit } if hit != Hit::Outside => moved(dock, x, now, p, out),
        DockEvent::PointerLeave | DockEvent::DndLeave => left(dock, now, p, out),
        DockEvent::PointerMove { .. } if dock.pointer_inside => left(dock, now, p, out),
        DockEvent::Frame => settle(dock, now, p),
        DockEvent::MenuClosed if !dock.menu_open => match dock.hover {
            Hover::Frozen {
                cause: FreezeCause::Menu,
                ..
            } => thaw(dock, now, out),
            _ => dock,
        },
        _ => dock,
    }
}

fn moved(
    dock: DockState,
    x: Cpx,
    now: Stamp,
    p: &DockParams,
    out: &mut Vec<DockEffect>,
) -> DockState {
    if matches!(dock.hover, Hover::Frozen { .. }) {
        return DockState {
            pointer_inside: true,
            ..dock
        };
    }
    let changed = x != dock.x;
    let hover = match dock.hover {
        Hover::Rest => Hover::Entering {
            from: Permille::ZERO,
            since: now,
        },
        Hover::Leaving { .. } => Hover::Entering {
            from: linear(&dock.hover, now, p),
            since: now,
        },
        Hover::Tracking { .. } if changed => Hover::Tracking { pending: true },
        other => other,
    };
    if hover != dock.hover || changed {
        out.push(DockEffect::RequestFrames);
    }
    DockState {
        pointer_inside: true,
        x,
        hover,
        ..dock
    }
}

fn left(dock: DockState, now: Stamp, p: &DockParams, out: &mut Vec<DockEffect>) -> DockState {
    let hover = match dock.hover {
        Hover::Entering { .. } | Hover::Tracking { .. } => {
            out.push(DockEffect::RequestFrames);
            Hover::Leaving {
                from: linear(&dock.hover, now, p),
                since: now,
            }
        }
        other => other,
    };
    DockState {
        pointer_inside: false,
        hover,
        ..dock
    }
}

fn settle(dock: DockState, now: Stamp, p: &DockParams) -> DockState {
    let hover = match dock.hover {
        Hover::Entering { .. } if linear(&dock.hover, now, p) == Permille::ONE => {
            Hover::Tracking { pending: true }
        }
        Hover::Leaving { .. } if linear(&dock.hover, now, p) == Permille::ZERO => Hover::Rest,
        other => other,
    };
    DockState { hover, ..dock }
}

#[cfg(test)]
#[path = "hover_tests.rs"]
mod tests;
