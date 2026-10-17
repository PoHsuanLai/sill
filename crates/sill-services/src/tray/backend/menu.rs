//! dbusmenu rows as the tray's menu entries, within the tree's limits: at most
//! [`MENU_ROWS_MAX`] rows in all and [`MENU_DEPTH_MAX`] levels, labels cut, icons of a sane size.

use sill_model::tray::{
    Check, Chord, MENU_DEPTH_MAX, MENU_ICON_MAX_BYTES, MENU_LABEL_MAX_CHARS, MENU_ROWS_MAX,
    TrayItemId, TrayMenuEntry, TrayMenuIcon, TrayMenuKind, plain_label, theme_icon_name,
};
use sill_model::units::{Availability, Visibility};
use sill_model::untrusted::for_display;
use system_tray::client::Client;
use system_tray::menu::{MenuItem, MenuType, ToggleState, ToggleType};

use super::items::destination_of;

pub(super) fn to_check(item: &MenuItem) -> Check {
    match item.toggle_type {
        ToggleType::CannotBeToggled => Check::None,
        ToggleType::Checkmark | ToggleType::Radio => match item.toggle_state {
            ToggleState::On => Check::Checked,
            ToggleState::Off => Check::Unchecked,
            ToggleState::Indeterminate => Check::Mixed,
        },
    }
}

/// `icon-data` (PNG bytes) when the row carries any, else `icon-name` when it is a plain theme
/// name ([`theme_icon_name`]).
pub(super) fn to_menu_icon(item: &MenuItem) -> TrayMenuIcon {
    match (item.icon_data.as_deref(), item.icon_name.as_deref()) {
        (Some(png), _) if !png.is_empty() && png.len() <= MENU_ICON_MAX_BYTES => {
            TrayMenuIcon::Png(png.to_vec())
        }
        (_, Some(name)) => theme_icon_name(name).map_or(TrayMenuIcon::None, |name| {
            TrayMenuIcon::Named(name.to_owned())
        }),
        _ => TrayMenuIcon::None,
    }
}

/// How many rows may still be taken from the tree being read.
#[derive(Debug, Clone, Copy)]
struct RowsLeft(usize);

/// `items` as entries: the first rows of each level, in order, while the tree's budget lasts, and
/// no level deeper than [`MENU_DEPTH_MAX`].
pub(super) fn to_menu_entries(items: &[MenuItem]) -> Vec<TrayMenuEntry> {
    entries(items, 1, &mut RowsLeft(MENU_ROWS_MAX))
}

fn entries(items: &[MenuItem], depth: usize, rows: &mut RowsLeft) -> Vec<TrayMenuEntry> {
    if depth > MENU_DEPTH_MAX {
        return Vec::new();
    }
    let mut taken = Vec::new();
    for item in items {
        let Some(left) = rows.0.checked_sub(1) else {
            break;
        };
        rows.0 = left;
        taken.push(entry(item, depth, rows));
    }
    taken
}

fn entry(item: &MenuItem, depth: usize, rows: &mut RowsLeft) -> TrayMenuEntry {
    let kind = match item.menu_type {
        MenuType::Separator => TrayMenuKind::Separator,
        MenuType::Standard => TrayMenuKind::Item {
            label: for_display(
                &plain_label(item.label.as_deref().unwrap_or_default()),
                MENU_LABEL_MAX_CHARS,
            ),
            enabled: if item.enabled {
                Availability::Enabled
            } else {
                Availability::Disabled
            },
            visibility: if item.visible {
                Visibility::Shown
            } else {
                Visibility::Hidden
            },
            check: to_check(item),
            icon: to_menu_icon(item),
            children: entries(&item.submenu, depth + 1, rows),
            shortcut: item
                .shortcut
                .as_ref()
                .and_then(|chords| chords.first())
                .map(|keys| Chord(keys.clone())),
            command: None,
        },
    };
    TrayMenuEntry { id: item.id, kind }
}

/// `destination`'s current full menu tree, straight from `Client`'s own cache: the `data`
/// feature (default-on) already applies every `Menu`/`MenuDiff` update onto it
/// (`system_tray::data::TrayItemMap::apply_update_event`), so this needs no diffing of its own.
pub(super) fn cached_menu(client: &Client, destination: &str) -> Vec<TrayMenuEntry> {
    let items = client.items();
    let items = items
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    items
        .get(destination)
        .and_then(|(_, menu)| menu.as_ref())
        .map(|menu| to_menu_entries(&menu.submenus))
        .unwrap_or_default()
}

/// The dbusmenu object path `destination`'s item advertised (`StatusNotifierItem::menu`),
/// needed for every dbusmenu call; `None` before it has arrived or if the item has no menu.
pub(super) fn menu_path_of(client: &Client, id: &TrayItemId) -> Option<String> {
    let destination = destination_of(id);
    let items = client.items();
    let items = items
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    items
        .get(destination)
        .and_then(|(item, _)| item.menu.clone())
}
