//! The dock's context menu (design/10 §10.3.7): windows, desktop actions, Options (Keep in
//! Dock, Open at Login, Show in Files), Hide, Quit / Force Quit; order per
//! `dock.context_menu_order`.
//!
//! `WindowsFirst` (the default, design/22's "plan order") is flat: windows, desktop actions,
//! Keep in Dock (reading Remove from Dock on a pinned app), Quit. `AppleOrder` is §10.3.7's:
//! the same, then Keep in Dock and Show in Files in an `Options` submenu, Hide and Quit. Open
//! at Login is not offered: writing `~/.config/autostart` is a service sill does not have yet.

use ds::{Availability, Check, MenuEntry, MenuTrail};
use sill_launcher::AppId;
use sill_services::{Activity, App, AppIndex, Toplevel, Toplevels, WindowState, match_app};
use sill_settings::DockMenuOrder;

use crate::surfaces::dock::items::{DockItem, ItemKind};
use crate::surfaces::dock::pick::DockPick;

/// One row of the menu, before it becomes quire `MenuEntry`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockMenuRow {
    Header(String),
    Window {
        title: String,
        active: Checked,
        minimized: Minimized,
    },
    DesktopAction {
        id: String,
        name: String,
    },
    KeepInDock(Checked),
    OpenAtLogin(Checked),
    ShowInFiles,
    Hide,
    Quit,
    ForceQuit,
    Separator,
}

/// A check mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checked {
    Checked,
    Unchecked,
}

/// Whether a listed window is minimized (prefixed with the diamond glyph).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Minimized {
    Minimized,
    Normal,
}

/// Whether Option is held while the menu is open (Quit reads Force Quit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionHeld {
    Held,
    Up,
}

/// The rows for `item`.
pub fn context_menu(
    item: &DockItem,
    app: Option<&App>,
    toplevels: &Toplevels,
    order: DockMenuOrder,
    option: OptionHeld,
) -> Vec<DockMenuRow> {
    if item.kind == ItemKind::Trash {
        return Vec::new();
    }
    let windows = menu_windows(item, app, toplevels);
    let running = !windows.is_empty();
    let window_rows = windows.iter().map(|w| DockMenuRow::Window {
        title: w.title.clone(),
        active: checked(w.activity == Activity::Active),
        minimized: match w.window {
            WindowState::Minimized => Minimized::Minimized,
            WindowState::Normal => Minimized::Normal,
        },
    });
    let actions =
        app.into_iter()
            .flat_map(|app| &app.actions)
            .map(|a| DockMenuRow::DesktopAction {
                id: a.id.clone(),
                name: a.name.clone(),
            });
    let pinned = checked(matches!(item.kind, ItemKind::Pinned(_)));
    let keep = item.kind.app_id().map(|_| DockMenuRow::KeepInDock(pinned));
    let options: Vec<DockMenuRow> = match order {
        DockMenuOrder::WindowsFirst => keep.into_iter().collect(),
        DockMenuOrder::AppleOrder => keep
            .into_iter()
            .chain(item.kind.app_id().map(|_| DockMenuRow::ShowInFiles))
            .collect(),
    };
    let quit = match option {
        OptionHeld::Held => DockMenuRow::ForceQuit,
        OptionHeld::Up => DockMenuRow::Quit,
    };
    let ending: Vec<DockMenuRow> = match (order, running) {
        (_, false) => Vec::new(),
        (DockMenuOrder::WindowsFirst, true) => vec![quit],
        (DockMenuOrder::AppleOrder, true) => vec![DockMenuRow::Hide, quit],
    };
    sections([window_rows.collect(), actions.collect(), options, ending])
}

/// The non-empty sections, a separator between each.
fn sections<const N: usize>(parts: [Vec<DockMenuRow>; N]) -> Vec<DockMenuRow> {
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

fn checked(on: bool) -> Checked {
    if on {
        Checked::Checked
    } else {
        Checked::Unchecked
    }
}

/// The windows the menu lists for `item`, most recent first: its app's, as `match_app`
/// against that app's entry alone decides, or the raw app id's for an unmatched tile.
pub fn menu_windows<'a>(
    item: &DockItem,
    app: Option<&App>,
    toplevels: &'a Toplevels,
) -> Vec<&'a Toplevel> {
    let one = AppIndex {
        apps: app.cloned().into_iter().collect(),
    };
    let belongs = |w: &Toplevel| match &item.kind {
        ItemKind::Pinned(id) | ItemKind::Running(id) => {
            w.app_id == id.0 || match_app(&w.app_id, None, &one).as_ref() == Some(id)
        }
        ItemKind::Unmatched(raw) => w.app_id == *raw,
        ItemKind::Trash => false,
    };
    toplevels.windows.iter().filter(|w| belongs(w)).collect()
}

/// `rows` as the ds menu's entries, each pick naming what it acts on. `windows` are the
/// windows `rows` lists, in the same order.
pub fn menu_entries(
    rows: &[DockMenuRow],
    item: &DockItem,
    windows: &[&Toplevel],
    order: DockMenuOrder,
) -> Vec<MenuEntry<DockPick>> {
    let app: Option<AppId> = item.kind.app_id().cloned();
    let keys: Vec<_> = windows.iter().map(|w| w.key).collect();
    let mut listed = windows.iter();
    let in_options = |row: &&DockMenuRow| {
        order == DockMenuOrder::AppleOrder
            && matches!(
                row,
                DockMenuRow::KeepInDock(_) | DockMenuRow::OpenAtLogin(_) | DockMenuRow::ShowInFiles
            )
    };
    let (options, rest): (Vec<&DockMenuRow>, Vec<&DockMenuRow>) = rows.iter().partition(in_options);
    // The submenu takes the place of its first row.
    let at = rows.iter().position(|row| in_options(&row));
    let submenu =
        (!options.is_empty()).then(|| options_submenu(&options, app.as_ref(), &keys, order));
    let before = at.map_or(rest.len(), |at| {
        rows[..at].iter().filter(|r| !in_options(r)).count()
    });
    let mut out: Vec<MenuEntry<DockPick>> = rest[..before]
        .iter()
        .filter_map(|row| row_entry(row, app.as_ref(), &keys, order, &mut listed))
        .collect();
    out.extend(submenu);
    out.extend(
        rest[before..]
            .iter()
            .filter_map(|row| row_entry(row, app.as_ref(), &keys, order, &mut listed)),
    );
    out
}

fn options_submenu(
    rows: &[&DockMenuRow],
    app: Option<&AppId>,
    keys: &[sill_launcher::WindowKey],
    order: DockMenuOrder,
) -> MenuEntry<DockPick> {
    let mut none = [].iter();
    MenuEntry::Submenu {
        title: "Options".to_owned(),
        tile: None,
        availability: Availability::Enabled,
        children: rows
            .iter()
            .filter_map(|row| row_entry(row, app, keys, order, &mut none))
            .collect(),
    }
}

/// One row's entry; `listed` hands each window row its window.
fn row_entry(
    row: &DockMenuRow,
    app: Option<&AppId>,
    keys: &[sill_launcher::WindowKey],
    order: DockMenuOrder,
    listed: &mut std::slice::Iter<&Toplevel>,
) -> Option<MenuEntry<DockPick>> {
    let owned = |f: fn(AppId) -> DockPick| app.cloned().map(f);
    let (title, value, check) = match row {
        DockMenuRow::Header(text) => return Some(MenuEntry::Header(text.clone())),
        DockMenuRow::Separator => return Some(MenuEntry::Separator),
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
            let check = (*active == Checked::Checked).then_some(Check::Checked);
            (title, DockPick::Activate(key), check)
        }
        DockMenuRow::DesktopAction { id, name } => (
            name.clone(),
            DockPick::Action {
                app: app?.clone(),
                action: id.clone(),
            },
            None,
        ),
        DockMenuRow::KeepInDock(pinned) => keep_entry(*pinned, order, app)?,
        DockMenuRow::OpenAtLogin(_) => return None,
        DockMenuRow::ShowInFiles => (
            "Show in Files".to_owned(),
            owned(DockPick::ShowInFiles)?,
            None,
        ),
        DockMenuRow::Hide => ("Hide".to_owned(), DockPick::Hide(keys.to_vec()), None),
        DockMenuRow::Quit => ("Quit".to_owned(), DockPick::Quit(keys.to_vec()), None),
        DockMenuRow::ForceQuit => ("Force Quit".to_owned(), owned(DockPick::ForceQuit)?, None),
    };
    Some(MenuEntry::Item {
        value,
        title,
        detail: None,
        tile: None,
        trail: MenuTrail::None,
        check,
        availability: Availability::Enabled,
    })
}

/// Keep in Dock: flat, it names what a pick does; in Options, a check mark says the state.
fn keep_entry(
    pinned: Checked,
    order: DockMenuOrder,
    app: Option<&AppId>,
) -> Option<(String, DockPick, Option<Check>)> {
    let app = app?.clone();
    let value = match pinned {
        Checked::Checked => DockPick::Unpin(app),
        Checked::Unchecked => DockPick::Pin(app),
    };
    Some(match (order, pinned) {
        (DockMenuOrder::WindowsFirst, Checked::Checked) => {
            ("Remove from Dock".to_owned(), value, None)
        }
        (DockMenuOrder::WindowsFirst, Checked::Unchecked) => {
            ("Keep in Dock".to_owned(), value, None)
        }
        (DockMenuOrder::AppleOrder, Checked::Checked) => {
            ("Keep in Dock".to_owned(), value, Some(Check::Checked))
        }
        (DockMenuOrder::AppleOrder, Checked::Unchecked) => ("Keep in Dock".to_owned(), value, None),
    })
}

#[cfg(test)]
#[path = "context_menu_tests.rs"]
mod tests;
