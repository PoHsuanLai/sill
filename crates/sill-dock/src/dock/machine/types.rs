//! The dock machine's vocabulary: its whole state, its events and its effects.

use ds::base::time::stamp::Stamp;
use shell_host::{BlurSpec, ExclusiveZone, RegionSpec};
use sill_model::units::Cpx;

use super::hover::Hover;
use super::label::Label;
use super::press::{ActivateMode, DropZone, Instance, Press};
use super::reveal::Reveal;

/// A tile, by the component's index-stable key (pins then running apps then Trash).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileId(pub u32);

/// A surface-local point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: Cpx,
    pub y: Cpx,
}

/// Which button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    Primary,
    Middle,
    Secondary,
}

/// The modifiers design/10 §10.3.6 distinguishes: Cmd is Super or Ctrl, Option is Alt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Modifier {
    #[default]
    None,
    Cmd,
    Alt,
    CmdAlt,
}

/// What a tile's app is doing, as `match_app` and the toplevel list say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileApp {
    NotRunning,
    /// Alive (D-Bus activatable) with no toplevel anywhere.
    NoWindows,
    AllMinimized,
    /// Has windows, not the active app.
    Inactive,
    /// The active app, with this many toplevels on the current workspace.
    Active {
        windows: u32,
    },
}

/// What the pointer is over, from the component's hit test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hit {
    Tile {
        tile: TileId,
        app: TileApp,
    },
    /// Inside the hit region, between or around tiles.
    Pill,
    Outside,
}

impl Hit {
    /// The tile under the pointer, if any.
    pub fn tile(self) -> Option<TileId> {
        match self {
            Hit::Tile { tile, .. } => Some(tile),
            Hit::Pill | Hit::Outside => None,
        }
    }
}

/// The keys the dock handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockKey {
    Escape,
}

/// Where the pointer is, and the last x it had inside (Leaving and Frozen keep drawing there).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pointer {
    #[default]
    Outside,
    Inside,
}

/// Whether a dock context menu is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuState {
    #[default]
    Closed,
    Open,
}

/// Spring-loading: a drag resting on a running app's tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Spring {
    #[default]
    Idle,
    Resting {
        tile: TileId,
        since: Stamp,
    },
}

/// The whole dock.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DockState {
    pub pointer: Pointer,
    /// The last pointer x inside the hit region.
    pub x: Cpx,
    pub hover: Hover,
    pub label: Label,
    pub press: Press,
    pub reveal: Reveal,
    pub menu: MenuState,
    pub spring: Spring,
}

impl DockState {
    /// A dock at rest; `Reveal::Hidden` when auto-hide is on.
    pub fn new(reveal: Reveal) -> DockState {
        DockState {
            reveal,
            ..DockState::default()
        }
    }
}

/// One input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockEvent {
    PointerEnter {
        at: Point,
        hit: Hit,
    },
    /// `zone` is where a drag would drop, from the frame's layout (unused unless dragging).
    PointerMove {
        at: Point,
        hit: Hit,
        zone: DropZone,
    },
    PointerLeave,
    Press {
        button: Button,
        at: Point,
        hit: Hit,
    },
    Release {
        button: Button,
        at: Point,
        hit: Hit,
        modifier: Modifier,
    },
    Key(DockKey),
    /// A drag-and-drop from another client moves over the dock.
    DndMotion {
        x: Cpx,
        hit: Hit,
    },
    DndLeave,
    Drop,
    MenuClosed,
    /// A `wl_surface.frame` callback.
    Frame,
    /// A one-shot timer asked for with [`DockEffect::WakeAt`] fired.
    Tick,
}

/// One thing the component must do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockEffect {
    /// Draw this frame from [`super::displayed`].
    Redraw,
    RequestFrames,
    StopFrames,
    /// Send [`DockEvent::Tick`] at this instant (or on the first frame after it).
    WakeAt(Stamp),
    SetInput(RegionSpec),
    SetBlur(BlurSpec),
    SetExclusive(ExclusiveZone),
    ShowLabel {
        tile: TileId,
    },
    HideLabel,
    OpenMenu {
        tile: TileId,
    },
    Launch {
        tile: TileId,
        instance: Instance,
    },
    Activate {
        tile: TileId,
        mode: ActivateMode,
    },
    ShowInFiles {
        tile: TileId,
    },
    Reorder {
        tile: TileId,
        to: u32,
    },
    Unpin {
        tile: TileId,
    },
    SpringOpen {
        tile: TileId,
    },
    /// Re-send every toplevel's minimize rectangle (debounced by the component).
    SetMinimizeRects,
}
