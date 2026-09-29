//! dbusmenu rows as the tray's menu entries.

use sill_model::tray::{Check, TrayItemId, TrayMenuEntry, TrayMenuKind};
use sill_model::units::{Availability, Visibility};
use system_tray::client::Client;
use system_tray::menu::{MenuItem, MenuType, ToggleState, ToggleType};

use super::items::destination_of;

pub(super) fn to_check(item: &MenuItem) -> Check {
    match item.toggle_type {
        ToggleType::CannotBeToggled => Check::None,
        ToggleType::Checkmark | ToggleType::Radio => match item.toggle_state {
            ToggleState::On => Check::Checked,
            // Indeterminate has no home in the frozen `Check` (None | Unchecked | Checked):
            // nearest is Unchecked, since neither reads as "on".
            ToggleState::Off | ToggleState::Indeterminate => Check::Unchecked,
        },
    }
}

pub(super) fn to_menu_entry(item: &MenuItem) -> TrayMenuEntry {
    let kind = match item.menu_type {
        MenuType::Separator => TrayMenuKind::Separator,
        MenuType::Standard => TrayMenuKind::Item {
            label: item.label.clone().unwrap_or_default(),
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
