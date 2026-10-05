//! dbusmenu rows as the tray's menu entries.

use sill_model::tray::{Check, TrayItemId, TrayMenuEntry, TrayMenuIcon, TrayMenuKind, plain_label};
use sill_model::units::{Availability, Visibility};
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

/// `icon-data` (PNG bytes) when the row carries any, else `icon-name`.
pub(super) fn to_menu_icon(item: &MenuItem) -> TrayMenuIcon {
    match (item.icon_data.as_deref(), item.icon_name.as_deref()) {
        (Some(png), _) if !png.is_empty() => TrayMenuIcon::Png(png.to_vec()),
        (_, Some(name)) if !name.is_empty() => TrayMenuIcon::Named(name.to_owned()),
        _ => TrayMenuIcon::None,
    }
}

pub(super) fn to_menu_entry(item: &MenuItem) -> TrayMenuEntry {
    let kind = match item.menu_type {
        MenuType::Separator => TrayMenuKind::Separator,
        MenuType::Standard => TrayMenuKind::Item {
            label: plain_label(item.label.as_deref().unwrap_or_default()),
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
            children: item.submenu.iter().map(to_menu_entry).collect(),
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
        .map(|menu| menu.submenus.iter().map(to_menu_entry).collect())
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
