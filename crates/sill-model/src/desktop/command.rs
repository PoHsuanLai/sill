//! What the shell asks of the desktop, and which backend can carry it.

use casement_protocols::vocab::PlaceTarget;
use porter_core::SpaceId;

use super::DesktopBackend;

/// A request to the desktop. A window is named by its [`super::WindowView::identifier`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopCommand {
    /// Bring a window to the front and give it the keyboard (restoring it when minimised).
    Activate(String),
    /// Ask a window to close.
    Close(String),
    /// Minimise a window.
    Minimise(String),
    /// Restore a minimised window.
    Unminimise(String),
    /// Tile, fill or centre a window.
    Place(String, PlaceTarget),
    /// Move a window to a Space.
    ToSpace(String, SpaceId),
    /// Show a Space.
    SwitchSpace(SpaceId),
    /// Say where a minimised window's tile is, as `(x, y, width, height)` in the coordinates
    /// of the dock's own surface.
    SetMinimiseTarget {
        /// The window.
        toplevel: String,
        /// The tile.
        rect: (i32, i32, u32, u32),
    },
}

/// Whether a backend can carry a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Supports {
    /// The backend carries it.
    Yes,
    /// The backend cannot; a surface leaves the control out.
    No,
}

/// Whether `backend` can carry `command`.
///
/// Wlr cannot place a window or move it between Spaces (no standard protocol does), and cannot
/// take a minimise target (the protocol needs the dock's own `wl_surface`, which lives on the
/// shell host's connection, so the command carries no surface to name). Cosmic has no
/// placement or minimise target either; it moves windows between workspaces.
pub fn supports(backend: DesktopBackend, command: &DesktopCommand) -> Supports {
    match backend {
        DesktopBackend::Quire => Supports::Yes,
        DesktopBackend::Wlr => wlr_supports(command),
        DesktopBackend::Cosmic => cosmic_supports(command),
    }
}

fn wlr_supports(command: &DesktopCommand) -> Supports {
    match command {
        DesktopCommand::Activate(_)
        | DesktopCommand::Close(_)
        | DesktopCommand::Minimise(_)
        | DesktopCommand::Unminimise(_)
        | DesktopCommand::SwitchSpace(_) => Supports::Yes,
        DesktopCommand::Place(..)
        | DesktopCommand::ToSpace(..)
        | DesktopCommand::SetMinimiseTarget { .. } => Supports::No,
    }
}

fn cosmic_supports(command: &DesktopCommand) -> Supports {
    match command {
        DesktopCommand::Activate(_)
        | DesktopCommand::Close(_)
        | DesktopCommand::Minimise(_)
        | DesktopCommand::Unminimise(_)
        | DesktopCommand::ToSpace(..)
        | DesktopCommand::SwitchSpace(_) => Supports::Yes,
        DesktopCommand::Place(..) | DesktopCommand::SetMinimiseTarget { .. } => Supports::No,
    }
}
