//! What the shell asks of the desktop, and which backend can carry it.

use casement_protocols::vocab::{Assign, OverviewMode, PlaceTarget, Toggle};
use porter_core::SpaceId;

use super::DesktopBackend;

/// A request to the desktop. A window is named by its [`super::WindowView::identifier`], an app
/// by its [`super::AppView::key`], a Space by its id and an output by its name.
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
    /// Forget where a minimised window's tile was.
    ClearMinimiseTarget(String),
    /// Show a window on every Space, the one it is on, or leave it where it opens.
    AssignSpaces(String, Assign),
    /// Bring an app forward and raise its windows.
    ActivateApp(String),
    /// Hide an app.
    HideApp(String),
    /// Show a hidden app.
    UnhideApp(String),
    /// Hide every app but this one.
    HideOtherApps(String),
    /// Ask an app to quit; it may refuse or ask the person first.
    QuitApp(String),
    /// End an app's processes.
    ForceQuitApp(String),
    /// Activate an app that has no windows; the desktop answers with
    /// [`super::ShellNote::Reopen`], and the dock decides what reopening means.
    ReopenApp(String),
    /// Add a Space on the output with this name.
    CreateSpace(String),
    /// Remove a Space; its windows move to the neighbouring desktop Space.
    RemoveSpace(SpaceId),
    /// Move a Space to a position on its output (0 is leftmost).
    MoveSpace {
        /// The Space.
        space: SpaceId,
        /// Where; 0 is leftmost.
        position: u32,
    },
    /// Open or close an overview.
    Overview {
        /// Which overview.
        mode: OverviewMode,
        /// Open (`On`) or close (`Off`).
        toggle: Toggle,
    },
    /// Move every window aside, or back.
    ShowDesktop(Toggle),
    /// The person picked an app in the switcher, by the index the desktop listed it at.
    PickSwitcher(u32),
    /// The person dismissed the switcher.
    CancelSwitcher,
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
/// Quire carries every command (the shell ones only on a connection that holds the shell role;
/// the arm refuses them otherwise). Wlr cannot place a window or move it between Spaces (no
/// standard protocol does), cannot hide or quit an app, and cannot take a minimise target on the
/// connection the arm runs on: the protocol needs the dock's own `wl_surface`, which lives on
/// the shell host's connection. Bare has no windows to carry anything.
pub fn supports(backend: DesktopBackend, command: &DesktopCommand) -> Supports {
    let carried = match backend {
        DesktopBackend::Quire => true,
        DesktopBackend::Wlr => wlr_carries(command),
        DesktopBackend::Bare => false,
    };
    match carried {
        true => Supports::Yes,
        false => Supports::No,
    }
}

fn wlr_carries(command: &DesktopCommand) -> bool {
    matches!(
        command,
        DesktopCommand::Activate(_)
            | DesktopCommand::Close(_)
            | DesktopCommand::Minimise(_)
            | DesktopCommand::Unminimise(_)
            | DesktopCommand::SwitchSpace(_)
    )
}
