//! The tray state's pure transition.

use super::{Tray, TrayEvent, TrayItem, TrayItemId};

/// `tray` after `event`. A change for an unknown item is dropped (it raced its removal).
pub fn step(tray: Tray, event: TrayEvent) -> Tray {
    let Tray { mut items } = tray;
    match event {
        TrayEvent::Registered(item) => match items.iter_mut().find(|i| i.id == item.id) {
            Some(known) => *known = item,
            None => items.push(item),
        },
        TrayEvent::Title { item, title } => edit(&mut items, &item, |i| i.title = title),
        TrayEvent::Icon { item, icon } => edit(&mut items, &item, |i| i.icon = icon),
        TrayEvent::Status { item, status } => edit(&mut items, &item, |i| i.status = status),
        TrayEvent::Menu { item, menu } => edit(&mut items, &item, |i| i.menu = menu),
        TrayEvent::Removed(item) => items.retain(|i| i.id != item),
    }
    Tray { items }
}

fn edit(items: &mut [TrayItem], id: &TrayItemId, change: impl FnOnce(&mut TrayItem)) {
    if let Some(item) = items.iter_mut().find(|i| &i.id == id) {
        change(item);
    }
}
