//! Rows as the ds menu's entries, each pick naming what it acts on.

use ds::prelude::*;
use sill_launcher::{AppId, WindowKey};
use sill_model::windows::{Toplevel, WindowState};
use sill_settings::{AutoHide, DockMenuOrder, Magnification};

use super::{Checked, DockMenuRow, Minimized};
use crate::dock::pick::{DockPick, LoginChange};

/// What the picks of one menu act on.
struct Target<'a> {
    app: Option<&'a AppId>,
    /// Every window the menu lists, in the order listed.
    keys: Vec<WindowKey>,
    /// The listed windows that are minimized.
    minimized: Vec<WindowKey>,
}

/// `rows` as the ds menu's entries. `app` is the tile's app, if it has one; `windows` are the
/// windows `rows` lists, in the same order.
pub fn menu_entries(
    rows: &[DockMenuRow],
    app: Option<&AppId>,
    windows: &[&Toplevel],
    order: DockMenuOrder,
) -> Vec<MenuItem<DockPick>> {
    let target = Target {
        app,
        keys: windows.iter().map(|w| w.key).collect(),
        minimized: windows
            .iter()
            .filter(|w| w.window == WindowState::Minimized)
            .map(|w| w.key)
            .collect(),
    };
    let mut listed = windows.iter();
    let in_options = |row: &&DockMenuRow| {
        order == DockMenuOrder::AppleOrder
            && matches!(
                row,
                DockMenuRow::KeepInDock(_)
                    | DockMenuRow::OpenAtLogin { .. }
                    | DockMenuRow::ShowInFiles
            )
    };
    let (options, rest): (Vec<&DockMenuRow>, Vec<&DockMenuRow>) = rows.iter().partition(in_options);
    // The submenu takes the place of its first row.
    let at = rows.iter().position(|row| in_options(&row));
    let submenu = (!options.is_empty()).then(|| options_submenu(&options, &target));
    let before = at.map_or(rest.len(), |at| {
        rows[..at].iter().filter(|r| !in_options(r)).count()
    });
    let mut out: Vec<MenuItem<DockPick>> = rest[..before]
        .iter()
        .filter_map(|row| row_entry(row, &target, &mut listed))
        .collect();
    out.extend(submenu);
    out.extend(
        rest[before..]
            .iter()
            .filter_map(|row| row_entry(row, &target, &mut listed)),
    );
    out
}

fn options_submenu(rows: &[&DockMenuRow], target: &Target<'_>) -> MenuItem<DockPick> {
    let mut none = [].iter();
    MenuItem::Submenu {
        title: "Options".to_owned(),
        image: None,
        availability: Availability::Enabled,
        children: rows
            .iter()
            .filter_map(|row| row_entry(row, target, &mut none))
            .collect(),
    }
}

/// One plain row of a menu.
struct Line {
    title: String,
    value: DockPick,
    check: Option<Check>,
    availability: Availability,
}

impl Line {
    fn new(title: impl Into<String>, value: DockPick) -> Line {
        Line {
            title: title.into(),
            value,
            check: None,
            availability: Availability::Enabled,
        }
    }

    fn marked(self, state: Checked) -> Line {
        Line {
            check: (state == Checked::Checked).then_some(Check::On),
            ..self
        }
    }

    fn available(self, availability: Availability) -> Line {
        Line {
            availability,
            ..self
        }
    }

    fn into_item(self) -> MenuItem<DockPick> {
        MenuItem::Item {
            value: self.value,
            title: self.title,
            image: None,
            key: None,
            check: self.check,
            availability: self.availability,
            hint: None,
            after: AfterPick::Close,
            text: Default::default(),
        }
    }
}

/// One row's entry; `listed` hands each window row its window.
fn row_entry(
    row: &DockMenuRow,
    target: &Target<'_>,
    listed: &mut std::slice::Iter<&Toplevel>,
) -> Option<MenuItem<DockPick>> {
    let app = target.app;
    let owned = |f: fn(AppId) -> DockPick| app.cloned().map(f);
    let line = match row {
        DockMenuRow::Header(text) => return Some(MenuItem::Header(text.clone())),
        DockMenuRow::Separator => return Some(MenuItem::Separator),
        DockMenuRow::Window {
            title,
            active,
            minimized,
        } => {
            let key = listed.next()?.key;
            let title = match minimized {
                Minimized::Minimized => format!("◆ {title}"),
                Minimized::Normal => title.clone(),
            };
            Line::new(title, DockPick::Activate(key)).marked(*active)
        }
        DockMenuRow::DesktopAction { id, name } => Line::new(
            name.clone(),
            DockPick::Action {
                app: app?.clone(),
                action: id.clone(),
            },
        ),
        DockMenuRow::ShowAllWindows => Line::new(
            "Show All Windows",
            DockPick::ShowAll {
                minimized: target.minimized.clone(),
                front: target.keys.first().copied(),
            },
        ),
        DockMenuRow::KeepInDock(pinned) => keep_line(*pinned, app?),
        DockMenuRow::OpenAtLogin { state, entry } => {
            let app = app?.clone();
            let change = match state {
                Checked::Checked => LoginChange::Off { app },
                Checked::Unchecked => LoginChange::On {
                    app,
                    entry: entry.clone(),
                },
            };
            Line::new("Open at Login", DockPick::OpenAtLogin(change)).marked(*state)
        }
        DockMenuRow::ShowInFiles => Line::new("Show in Files", owned(DockPick::ShowInFiles)?),
        DockMenuRow::Hide => Line::new("Hide", DockPick::Hide(target.keys.clone())),
        DockMenuRow::Quit => Line::new("Quit", DockPick::Quit(target.keys.clone())),
        DockMenuRow::ForceQuit => Line::new("Force Quit", owned(DockPick::ForceQuit)?),
        DockMenuRow::OpenTrash(files) => Line::new("Open", DockPick::OpenTrash(files.clone())),
        DockMenuRow::EmptyTrash(availability) => {
            Line::new("Empty Trash", DockPick::EmptyTrash).available(*availability)
        }
        DockMenuRow::Hiding(AutoHide::Off) => Line::new("Turn Hiding On", DockPick::ToggleHiding),
        DockMenuRow::Hiding(AutoHide::On) => Line::new("Turn Hiding Off", DockPick::ToggleHiding),
        DockMenuRow::Magnifying(Magnification::Off) => {
            Line::new("Turn Magnification On", DockPick::ToggleMagnification)
        }
        DockMenuRow::Magnifying(Magnification::On) => {
            Line::new("Turn Magnification Off", DockPick::ToggleMagnification)
        }
        DockMenuRow::DockSettings => Line::new("Dock Settings…", DockPick::OpenDockSettings),
    };
    Some(line.into_item())
}

/// Keep in Dock: the title names what a pick does.
fn keep_line(pinned: Checked, app: &AppId) -> Line {
    match pinned {
        Checked::Checked => Line::new("Remove from Dock", DockPick::Unpin(app.clone())),
        Checked::Unchecked => Line::new("Keep in Dock", DockPick::Pin(app.clone())),
    }
}
