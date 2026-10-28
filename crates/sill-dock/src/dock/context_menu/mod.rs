//! The dock's context menus (design/10 §10.3.7-10.3.9), three kinds, each built as rows first
//! (pure, table-tested) and then turned into the ds menu's entries:
//!
//! - [`app`]: an app tile's: windows, desktop actions, Options (Keep in Dock, Open at Login, Show
//!   in Files), Show All Windows, Hide, Quit / Force Quit.
//! - [`trash`]: the Trash tile's: Open and Empty Trash.
//! - [`divider`]: the dock's own, on its separator or empty background: hiding, magnification and
//!   Dock Settings.
//!
//! The app tile's order is §10.3.7's.

use std::path::PathBuf;

use ds::prelude::Availability;
use sill_launcher::AppId;
use sill_model::apps::{App, AppIndex};
use sill_model::windows::{Toplevel, Toplevels, app_of};
use sill_settings::OnOff;

use crate::dock::items::{DockItem, ItemKind};

pub mod app;
pub mod divider;
mod entries;
pub mod trash;

pub use entries::menu_entries;

/// One row of a menu, before it becomes quire `MenuEntry`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockMenuRow {
    Header(String),
    Window {
        title: String,
        active: Checked,
        /// Whether the window is minimized (prefixed with the diamond glyph).
        minimized: bool,
    },
    DesktopAction {
        id: String,
        name: String,
    },
    ShowAllWindows,
    KeepInDock(Checked),
    /// Open at Login, with the app's desktop entry that turning it on copies.
    OpenAtLogin {
        state: Checked,
        entry: PathBuf,
    },
    ShowInFiles,
    Hide,
    Quit,
    ForceQuit,
    Separator,
    /// The Trash tile's Open, in this file manager.
    OpenTrash(AppId),
    EmptyTrash(Availability),
    /// Turn Hiding On / Off, by the state it is in now.
    Hiding(OnOff),
    /// Turn Magnification On / Off, by the state it is in now.
    Magnifying(OnOff),
    DockSettings,
}

/// A check mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checked {
    Checked,
    Unchecked,
}

/// Whether Option is held while the menu is open (Quit reads Force Quit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionHeld {
    Held,
    Up,
}

/// The non-empty sections, a separator between each.
pub(super) fn sections<const N: usize>(parts: [Vec<DockMenuRow>; N]) -> Vec<DockMenuRow> {
    parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .enumerate()
        .flat_map(|(i, part)| {
            (i > 0)
                .then_some(DockMenuRow::Separator)
                .into_iter()
                .chain(part)
        })
        .collect()
}

/// The windows the menu lists for `item`, most recent first: its app's, as `app_of`
/// against that app's entry alone decides, or the raw app id's for an unmatched tile.
pub fn menu_windows<'a>(
    item: &DockItem,
    app: Option<&App>,
    toplevels: &'a Toplevels,
) -> Vec<&'a Toplevel> {
    let one = AppIndex::new(app.cloned().into_iter().collect());
    let belongs = |w: &Toplevel| match &item.kind {
        ItemKind::Pinned(id) | ItemKind::Running(id) => app_of(w, &one).id() == *id,
        ItemKind::Unmatched(raw) => w.app_id == *raw,
        ItemKind::Trash => false,
    };
    toplevels.windows.iter().filter(|w| belongs(w)).collect()
}

#[cfg(test)]
mod tests;
