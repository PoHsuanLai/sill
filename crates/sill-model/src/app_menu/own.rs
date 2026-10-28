//! The rows the shell makes itself in the app-name menu, from what the compositor says about the
//! app's windows and never from anything the app sends: raise or minimise a window, Hide, Hide
//! Others, Show All, Quit. Raising, hiding and showing only change what is on screen (`Read`); Quit cannot be
//! undone and an app's save prompt is not guaranteed, so it is `Destructive`.

use prov::Effect;
use sill_launcher::WindowKey;

use super::{ServedMenu, Titles, titles};
use crate::windows::{Toplevels, WindowState, WlCommand};

/// One thing the shell does for an app's windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OwnItem {
    /// Bring one window forward.
    Raise(WindowKey),
    /// Minimise one window.
    Minimise(WindowKey),
    /// Minimise every window of the app.
    Hide,
    /// Minimise every window of every other app.
    HideOthers,
    /// Bring every minimised window back.
    ShowAll,
    /// Ask every window of the app to close.
    Quit,
}

impl OwnItem {
    /// What doing it costs: the effect the gate would weigh.
    pub fn effect(self) -> Effect {
        match self {
            OwnItem::Raise(_)
            | OwnItem::Minimise(_)
            | OwnItem::Hide
            | OwnItem::HideOthers
            | OwnItem::ShowAll => Effect::Read,
            OwnItem::Quit => Effect::Destructive,
        }
    }
}

/// One row of the shell's own part of the app-name menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnRow {
    pub item: OwnItem,
    pub label: String,
}

/// Rows with no rule between them.
pub type OwnGroup = Vec<OwnRow>;

/// Whether the shell offers Quit: Quit is the app's own when its menu has an app menu, and the
/// shell offers it otherwise.
pub fn quit_offered(served: Option<&ServedMenu>) -> bool {
    quit_offered_in(served.map(titles).as_ref())
}

/// As [`quit_offered`], for titles already made.
pub fn quit_offered_in(made: Option<&Titles>) -> bool {
    made.and_then(|made| made.app.as_ref()).is_none()
}

/// The shell's own groups for the app called `name` with `windows`: its windows, then Minimise (the
/// first window), Hide, Hide Others and Show All, then Quit when `offer_quit`.
pub fn own_groups(name: &str, windows: &[(WindowKey, String)], offer_quit: bool) -> Vec<OwnGroup> {
    let row = |item, label: String| OwnRow { item, label };
    let raise: OwnGroup = windows
        .iter()
        .map(|(key, title)| row(OwnItem::Raise(*key), title.clone()))
        .collect();
    let minimise = windows
        .first()
        .map(|(key, _)| row(OwnItem::Minimise(*key), "Minimise".to_owned()));
    let hide: OwnGroup = minimise
        .into_iter()
        .chain([
            row(OwnItem::Hide, format!("Hide {name}")),
            row(OwnItem::HideOthers, "Hide Others".to_owned()),
            row(OwnItem::ShowAll, "Show All".to_owned()),
        ])
        .collect();
    let quit = match offer_quit {
        true => vec![row(OwnItem::Quit, format!("Quit {name}"))],
        false => Vec::new(),
    };
    [raise, hide, quit]
        .into_iter()
        .filter(|group| !group.is_empty())
        .collect()
}

/// The compositor commands `item` is, for the app whose windows have `app_id`.
pub fn commands(item: OwnItem, app_id: &str, toplevels: &Toplevels) -> Vec<WlCommand> {
    let windows = toplevels.windows.iter();
    match item {
        OwnItem::Raise(key) => vec![WlCommand::Activate(key)],
        OwnItem::Minimise(key) => vec![WlCommand::Minimize(key)],
        OwnItem::Hide => windows
            .filter(|w| w.app_id == app_id && w.window == WindowState::Normal)
            .map(|w| WlCommand::Minimize(w.key))
            .collect(),
        OwnItem::HideOthers => windows
            .filter(|w| w.app_id != app_id && w.window == WindowState::Normal)
            .map(|w| WlCommand::Minimize(w.key))
            .collect(),
        OwnItem::ShowAll => windows
            .filter(|w| w.window == WindowState::Minimized)
            .map(|w| WlCommand::Unminimize(w.key))
            .collect(),
        OwnItem::Quit => windows
            .filter(|w| w.app_id == app_id)
            .map(|w| WlCommand::Close(w.key))
            .collect(),
    }
}
