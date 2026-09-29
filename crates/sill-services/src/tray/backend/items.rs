//! SNI items as the tray's items: ids, icons and status.

use sill_model::tray::{Pixmap, Tray, TrayEvent, TrayIcon, TrayItem, TrayItemId, TrayStatus, step};
use system_tray::client::Client;
use system_tray::item::{IconPixmap as SniPixmap, Status as SniStatus, StatusNotifierItem};

use super::host::ITEM_PATH;

/// Every item already on the bus when the client connected, in the order `Client` returned
/// them.
pub(super) fn seed(client: &Client) -> Tray {
    let mut state = Tray::default();
    let items = client.items();
    let items = items
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (destination, (item, _menu)) in items.iter() {
        state = step(
            state,
            TrayEvent::Registered(to_tray_item(destination, item)),
        );
    }
    state
}

/// This item's id: `Client`'s events name it by bus destination alone (module doc).
pub(super) fn item_id(destination: &str) -> TrayItemId {
    TrayItemId(format!("{destination}{ITEM_PATH}"))
}

/// The bus destination an id names — the inverse of [`item_id`].
pub(super) fn destination_of(id: &TrayItemId) -> &str {
    id.0.strip_suffix(ITEM_PATH).unwrap_or(&id.0)
}

pub(super) fn to_tray_item(destination: &str, item: &StatusNotifierItem) -> TrayItem {
    TrayItem {
        id: item_id(destination),
        title: item.title.clone().unwrap_or_else(|| item.id.clone()),
        icon: to_tray_icon(item.icon_name.as_deref(), item.icon_pixmap.as_deref()),
        status: to_tray_status(item.status),
        menu: Vec::new(),
    }
}

/// "Visualizations are encouraged to prefer icon names over icon pixmaps if both are
/// available" (`system_tray::item::StatusNotifierItem::icon_name` doc); among pixmaps, the
/// largest one offered ([`Pixmap`]'s own doc comment).
pub(super) fn to_tray_icon(name: Option<&str>, pixmaps: Option<&[SniPixmap]>) -> TrayIcon {
    if let Some(name) = name.filter(|name| !name.is_empty()) {
        return TrayIcon::Named(name.to_owned());
    }
    let Some(largest) = pixmaps
        .unwrap_or_default()
        .iter()
        .max_by_key(|pixmap| i64::from(pixmap.width) * i64::from(pixmap.height))
    else {
        return TrayIcon::None;
    };
    TrayIcon::Pixmap(Pixmap {
        width: largest.width.max(0) as u32,
        height: largest.height.max(0) as u32,
        argb32: largest.pixels.clone(),
    })
}

pub(super) fn to_tray_status(status: SniStatus) -> TrayStatus {
    match status {
        SniStatus::Passive => TrayStatus::Passive,
        SniStatus::Active | SniStatus::Unknown => TrayStatus::Active,
        SniStatus::NeedsAttention => TrayStatus::NeedsAttention,
    }
}
