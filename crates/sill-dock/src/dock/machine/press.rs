//! Presses, clicks, drags and spring-loading (design/10 §10.3.6-10.3.8, §10.4 "Press").

use ds::base::time::stamp::Stamp;

use super::hover::{self, FreezeCause, Hover};
use super::params::DockParams;
use super::types::{
    Button, DockEffect, DockEvent, DockKey, DockState, Hit, Modifier, Point, Spring, TileApp,
    TileId,
};

/// Where a dragged tile would land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropZone {
    /// Insert at this slot.
    Slot(u32),
    /// Beyond the remove threshold: release unpins.
    Remove,
}

/// Launch into the running instance, or a new one (middle click).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instance {
    Existing,
    New,
}

/// How a click activates a running app (design/10 §10.3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivateMode {
    /// Activate its most recent toplevel.
    Raise,
    /// Unminimize and activate the most recent toplevel.
    Unminimize,
    /// `org.freedesktop.Application.Activate` (running, no toplevel).
    Reopen,
    /// Activate, then minimize the previously active app (Option-click).
    HideCurrent,
    /// Activate, then minimize every other app here (Option-Cmd-click).
    HideOthers,
}

/// The primary button's phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Press {
    #[default]
    Up,
    Down {
        tile: TileId,
        at: Point,
        button: Button,
        since: Stamp,
    },
    Dragging {
        tile: TileId,
        origin: Point,
        over: DropZone,
    },
}

/// The press half of [`super::step`].
pub(crate) fn step(
    dock: DockState,
    event: &DockEvent,
    now: Stamp,
    p: &DockParams,
    out: &mut Vec<DockEffect>,
) -> DockState {
    match (dock.press, *event) {
        (
            Press::Up,
            DockEvent::Press {
                button,
                at,
                hit: Hit::Tile { tile, .. },
            },
        ) => pressed(dock, tile, button, at, now, p, out),
        (
            Press::Up,
            DockEvent::Press {
                button: Button::Secondary,
                at,
                hit: Hit::Pill,
            },
        ) => open_divider_menu(dock, at, now, p, out),
        (
            Press::Down {
                tile,
                at: origin,
                button: Button::Primary,
                ..
            },
            DockEvent::PointerMove { at, zone, .. },
        ) if manhattan(origin, at) > p.drag_threshold.0 => {
            out.push(DockEffect::RequestFrames);
            dragged(
                DockState {
                    press: Press::Dragging {
                        tile,
                        origin,
                        over: zone,
                    },
                    ..dock
                },
                zone,
                now,
                p,
                out,
            )
        }
        (Press::Dragging { tile, origin, .. }, DockEvent::PointerMove { zone, .. }) => dragged(
            DockState {
                press: Press::Dragging {
                    tile,
                    origin,
                    over: zone,
                },
                ..dock
            },
            zone,
            now,
            p,
            out,
        ),
        (Press::Down { tile, button, .. }, DockEvent::Release { hit, modifier, .. }) => {
            let dock = DockState {
                press: Press::Up,
                ..dock
            };
            match hit {
                Hit::Tile { tile: t, app } if t == tile => {
                    clicked(dock, tile, button, app, modifier, out)
                }
                _ => dock,
            }
        }
        (Press::Dragging { tile, over, .. }, DockEvent::Release { .. }) => {
            match over {
                DropZone::Remove => out.push(DockEffect::Unpin { tile }),
                DropZone::Slot(to) => out.push(DockEffect::Reorder { tile, to }),
            }
            hover::thaw(
                DockState {
                    press: Press::Up,
                    ..dock
                },
                now,
                out,
            )
        }
        (Press::Dragging { .. }, DockEvent::Key(DockKey::Escape)) => {
            out.push(DockEffect::RequestFrames);
            hover::thaw(
                DockState {
                    press: Press::Up,
                    ..dock
                },
                now,
                out,
            )
        }
        (
            Press::Down {
                tile,
                button: Button::Primary,
                since,
                ..
            },
            DockEvent::Frame | DockEvent::Tick,
        ) if now.since(since) >= p.hold_ms.0 => open_menu(
            DockState {
                press: Press::Up,
                ..dock
            },
            tile,
            now,
            p,
            out,
        ),
        (_, DockEvent::MenuClosed) => DockState {
            menu_open: false,
            ..dock
        },
        (_, DockEvent::DndMotion { hit, .. }) => DockState {
            spring: spring(dock.spring, hit, now, p, out),
            ..dock
        },
        (_, DockEvent::DndLeave | DockEvent::Drop) => DockState {
            spring: Spring::Idle,
            ..dock
        },
        (_, DockEvent::Frame | DockEvent::Tick) => match dock.spring {
            Spring::Resting { tile, since } if now.since(since) >= p.spring_load_ms.0 => {
                out.push(DockEffect::SpringOpen { tile });
                DockState {
                    spring: Spring::Idle,
                    ..dock
                }
            }
            _ => dock,
        },
        _ => dock,
    }
}

fn pressed(
    dock: DockState,
    tile: TileId,
    button: Button,
    at: Point,
    now: Stamp,
    p: &DockParams,
    out: &mut Vec<DockEffect>,
) -> DockState {
    match button {
        Button::Secondary => open_menu(dock, tile, now, p, out),
        Button::Primary => {
            out.push(DockEffect::WakeAt(now.after(p.hold_ms)));
            DockState {
                press: Press::Down {
                    tile,
                    at,
                    button,
                    since: now,
                },
                ..dock
            }
        }
        Button::Middle => DockState {
            press: Press::Down {
                tile,
                at,
                button,
                since: now,
            },
            ..dock
        },
    }
}

fn open_menu(
    dock: DockState,
    tile: TileId,
    now: Stamp,
    p: &DockParams,
    out: &mut Vec<DockEffect>,
) -> DockState {
    out.push(DockEffect::OpenMenu { tile });
    let dock = hover::freeze(dock, FreezeCause::Menu, now, p);
    DockState {
        menu_open: true,
        ..dock
    }
}

fn open_divider_menu(
    dock: DockState,
    at: Point,
    now: Stamp,
    p: &DockParams,
    out: &mut Vec<DockEffect>,
) -> DockState {
    out.push(DockEffect::OpenDividerMenu { at });
    let dock = hover::freeze(dock, FreezeCause::Menu, now, p);
    DockState {
        menu_open: true,
        ..dock
    }
}

/// While dragging: freeze magnification once the ghost is beyond the remove threshold, and
/// resume when it comes back.
fn dragged(
    dock: DockState,
    zone: DropZone,
    now: Stamp,
    p: &DockParams,
    out: &mut Vec<DockEffect>,
) -> DockState {
    match (zone, dock.hover) {
        (DropZone::Remove, _) => hover::freeze(dock, FreezeCause::Drag, now, p),
        (
            DropZone::Slot(_),
            Hover::Frozen {
                cause: FreezeCause::Drag,
                ..
            },
        ) => hover::thaw(dock, now, out),
        _ => dock,
    }
}

fn manhattan(a: Point, b: Point) -> i32 {
    (a.x.0 - b.x.0).abs() + (a.y.0 - b.y.0).abs()
}

/// The effect of a click released on the tile it pressed (design/10 §10.3.6).
fn clicked(
    dock: DockState,
    tile: TileId,
    button: Button,
    app: TileApp,
    modifier: Modifier,
    out: &mut Vec<DockEffect>,
) -> DockState {
    match (button, modifier, app) {
        (Button::Middle, _, _) => {
            out.push(DockEffect::Launch {
                tile,
                instance: Instance::New,
            });
            dock
        }
        (_, Modifier::Cmd, _) => {
            out.push(DockEffect::ShowInFiles { tile });
            dock
        }
        (_, _, TileApp::NotRunning) => {
            out.push(DockEffect::Launch {
                tile,
                instance: Instance::Existing,
            });
            dock
        }
        (_, Modifier::Alt, _) => activate(dock, tile, ActivateMode::HideCurrent, out),
        (_, Modifier::CmdAlt, _) => activate(dock, tile, ActivateMode::HideOthers, out),
        (_, Modifier::None, TileApp::NoWindows) => activate(dock, tile, ActivateMode::Reopen, out),
        (_, Modifier::None, TileApp::AllMinimized) => {
            activate(dock, tile, ActivateMode::Unminimize, out)
        }
        (_, Modifier::None, TileApp::Inactive) => activate(dock, tile, ActivateMode::Raise, out),
        // Clicking the active app does nothing.
        (_, Modifier::None, TileApp::Active { .. }) => dock,
    }
}

fn activate(
    dock: DockState,
    tile: TileId,
    mode: ActivateMode,
    out: &mut Vec<DockEffect>,
) -> DockState {
    out.push(DockEffect::Activate { tile, mode });
    dock
}

/// Spring-loading (design/10 §10.3.8): a drag resting on a running app's tile.
fn spring(
    spring: Spring,
    hit: Hit,
    now: Stamp,
    p: &DockParams,
    out: &mut Vec<DockEffect>,
) -> Spring {
    match (spring, hit) {
        (Spring::Resting { tile, .. }, Hit::Tile { tile: t, .. }) if t == tile => spring,
        (_, Hit::Tile { tile, app }) if app != TileApp::NotRunning => {
            out.push(DockEffect::WakeAt(now.after(p.spring_load_ms)));
            Spring::Resting { tile, since: now }
        }
        _ => Spring::Idle,
    }
}

#[cfg(test)]
#[path = "press_tests.rs"]
mod tests;
