//! A tray item's dbusmenu rows as the bar's menu entries, so the item's menu is one more bar
//! menu: the same popup, open delay, safe triangle and hover switching as the others (the
//! submenus open inside the popup's `Menu`, which times them). Pure: a row's icon is resolved by
//! the caller's function, since a theme lookup is an effect.

use ds::components::menus::item::item::MenuImage;
use ds::prelude::Check as DsCheck;
use ds::prelude::*;
use sill_model::tray::{Check as TrayCheck, TrayItemId, TrayMenuEntry, TrayMenuIcon, TrayMenuKind};
use sill_model::units::{Availability as TrayAvailability, Visibility};

use super::pick::BarPick;

/// How a row's icon becomes the image at its start; `None` draws none.
pub(crate) type RowIcon<'a> = &'a dyn Fn(&TrayMenuIcon) -> Option<MenuImage>;

/// Whether `rows` has a row to show: an item with nothing but hidden rows and separators has no
/// menu to open.
pub(crate) fn has_rows(rows: &[TrayMenuEntry]) -> bool {
    rows.iter().any(|row| match &row.kind {
        TrayMenuKind::Separator => false,
        TrayMenuKind::Item { visibility, .. } => *visibility == Visibility::Shown,
    })
}

/// `rows` of `item`'s dbusmenu as menu entries: hidden rows dropped, a row with shown children a
/// submenu (whatever its own check), and rules only between rows.
pub(crate) fn menu_items(
    item: &TrayItemId,
    rows: &[TrayMenuEntry],
    icon: RowIcon,
) -> Vec<MenuItem<BarPick>> {
    let entries = rows
        .iter()
        .filter_map(|row| entry(item, row, icon))
        .collect();
    between_rules(entries)
}

fn entry(item: &TrayItemId, row: &TrayMenuEntry, icon: RowIcon) -> Option<MenuItem<BarPick>> {
    let TrayMenuKind::Item {
        label,
        enabled,
        visibility,
        check,
        icon: row_icon,
        children,
    } = &row.kind
    else {
        return Some(MenuItem::Separator);
    };
    if *visibility == Visibility::Hidden {
        return None;
    }
    let availability = match enabled {
        TrayAvailability::Enabled => Availability::Enabled,
        TrayAvailability::Disabled => Availability::Disabled,
    };
    let image = icon(row_icon);
    let children = menu_items(item, children, icon);
    if !children.is_empty() {
        return Some(MenuItem::Submenu {
            title: label.clone(),
            image,
            availability,
            children,
        });
    }
    Some(MenuItem::Item {
        value: BarPick::Tray {
            item: item.clone(),
            entry: row.id,
        },
        title: label.clone(),
        image,
        key: None,
        hint: None,
        check: to_ds_check(*check),
        availability,
        after: AfterPick::Close,
    })
}

fn to_ds_check(check: TrayCheck) -> Option<DsCheck> {
    match check {
        TrayCheck::None => None,
        TrayCheck::Unchecked => Some(DsCheck::Off),
        TrayCheck::Checked => Some(DsCheck::On),
        TrayCheck::Mixed => Some(DsCheck::Mixed),
    }
}

/// `entries` without a rule at either end or two rules in a row (what dropping the hidden rows
/// between them leaves).
fn between_rules(entries: Vec<MenuItem<BarPick>>) -> Vec<MenuItem<BarPick>> {
    let mut kept: Vec<MenuItem<BarPick>> = Vec::with_capacity(entries.len());
    for entry in entries {
        let rule = matches!(entry, MenuItem::Separator);
        let after_rule = matches!(kept.last(), Some(MenuItem::Separator) | None);
        if !(rule && after_rule) {
            kept.push(entry);
        }
    }
    if matches!(kept.last(), Some(MenuItem::Separator)) {
        kept.pop();
    }
    kept
}

#[cfg(test)]
#[path = "tray_menu_tests.rs"]
mod tests;
