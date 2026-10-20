//! What a window or workspace ask comes to against the compositor's state: the commands to send,
//! or the text to answer. Pure: the service reads the state, sends the commands and says how it
//! went. A window or workspace the state does not hold is a fault, never a guess at what was meant.

use sill_launcher::WindowKey;

use super::listing::{windows_text, workspaces_text};
use super::window_ask::{WindowAsk, WorkspaceRef};
use crate::app_menu::{OwnItem, own_commands, window_of};
use crate::ids::WorkspaceKey;
use crate::windows::{Toplevel, WindowModel, WlCommand, Workspaces};

/// Why an ask was not done. The words never carry the argument's own text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFault {
    /// No window has that id.
    NoSuchWindow,
    /// No workspace has that id or index.
    NoSuchWorkspace,
    /// No window belongs to that app.
    NoSuchApp,
}

/// What an ask comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowAnswer {
    /// Text to hand back: a listing.
    Listing(String),
    /// Requests to the compositor, in order (none when there is nothing to change).
    Commands(Vec<WlCommand>),
}

/// The answer to `ask` against `wl`.
pub fn plan(ask: &WindowAsk, wl: &WindowModel) -> Result<WindowAnswer, WindowFault> {
    let toplevels = &wl.toplevels;
    let own = |item: OwnItem, app_id: &str| own_commands(item, app_id, toplevels);
    let commands = match ask {
        WindowAsk::List => return Ok(WindowAnswer::Listing(windows_text(wl))),
        WindowAsk::WorkspaceList => {
            return Ok(WindowAnswer::Listing(workspaces_text(&wl.workspaces)));
        }
        WindowAsk::Focus { window } => own(OwnItem::Raise(key_of(wl, window)?), ""),
        WindowAsk::Minimise { window } => own(OwnItem::Minimise(key_of(wl, window)?), ""),
        WindowAsk::Close { window } => vec![WlCommand::Close(key_of(wl, window)?)],
        WindowAsk::MoveToWorkspace { window, to } => vec![WlCommand::MoveToplevelToWorkspace {
            toplevel: key_of(wl, window)?,
            workspace: workspace_key(&wl.workspaces, to)?,
        }],
        WindowAsk::Hide { app } => own(OwnItem::Hide, &app_id(wl, app.as_deref())?),
        WindowAsk::HideOthers { app } => own(OwnItem::HideOthers, &app_id(wl, app.as_deref())?),
        WindowAsk::ShowAll => own(OwnItem::ShowAll, ""),
        WindowAsk::WorkspaceGo(to) => {
            vec![WlCommand::ActivateWorkspace(workspace_key(
                &wl.workspaces,
                to,
            )?)]
        }
    };
    Ok(WindowAnswer::Commands(commands))
}

/// The window whose id is exactly `id`: the listing's id, never an app id or a title.
pub fn window_by_id<'a>(wl: &'a WindowModel, id: &str) -> Option<&'a Toplevel> {
    let key = id.trim().parse::<u64>().ok()?;
    wl.toplevels.windows.iter().find(|w| w.key.0 == key)
}

fn key_of(wl: &WindowModel, id: &str) -> Result<WindowKey, WindowFault> {
    window_by_id(wl, id)
        .map(|w| w.key)
        .ok_or(WindowFault::NoSuchWindow)
}

/// The app id an app ask is about: the one named when some window has it, else the focused
/// window's.
fn app_id(wl: &WindowModel, named: Option<&str>) -> Result<String, WindowFault> {
    match named {
        Some(named) => wl
            .toplevels
            .windows
            .iter()
            .find(|w| w.app_id == named)
            .map(|w| w.app_id.clone())
            .ok_or(WindowFault::NoSuchApp),
        None => window_of(&wl.toplevels, None)
            .map(|w| w.app_id.clone())
            .ok_or(WindowFault::NoSuchApp),
    }
}

/// The workspace `to` names: an id any group has, or a position in the main group's order.
fn workspace_key(workspaces: &Workspaces, to: &WorkspaceRef) -> Result<WorkspaceKey, WindowFault> {
    let found = match to {
        WorkspaceRef::Id(id) => {
            let key = id.trim().parse::<u64>().ok();
            workspaces
                .groups
                .iter()
                .flat_map(|g| &g.workspaces)
                .find(|w| Some(w.key.0) == key)
        }
        WorkspaceRef::Index(at) => workspaces
            .main_group()
            .and_then(|g| g.workspaces.get(usize::try_from(*at).ok()?.checked_sub(1)?)),
    };
    found.map(|w| w.key).ok_or(WindowFault::NoSuchWorkspace)
}
