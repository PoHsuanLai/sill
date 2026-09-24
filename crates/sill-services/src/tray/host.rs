//! The SNI host loop: `system_tray::client::Client` events mapped to [`TrayEvent`], `Control`
//! commands mapped to `Client` calls.
//!
//! `system_tray::client::Client::new()` registers the `StatusNotifierWatcher` itself when the
//! bus has none yet (`StatusNotifierWatcher::attach_to`: it tries to own
//! `org.kde.StatusNotifierWatcher` and quietly defers when a desktop (KDE) already owns it), so
//! nothing here duplicates that.
//!
//! `Client`'s public API tracks every item by its bus **destination** alone (`:1.42`), not the
//! `<destination><path>` pair the frozen [`TrayItemId`] doc comment describes: `Client::
//! activate`'s notifier-item proxy always targets the constant `/StatusNotifierItem` path,
//! regardless of where an item actually registered (`system-tray` 0.8.9, `client.rs`
//! `get_notifier_item_proxy`). So every id here is `<destination>/StatusNotifierItem`, which is
//! both the doc comment's shape and the only path `Client::activate` can reach — the test
//! fixtures in `model.rs` already use exactly this shape (`":1.{name}/StatusNotifierItem"`).

use system_tray::client::{ActivateRequest, Client, Event, UpdateEvent};
use system_tray::item::{IconPixmap as SniPixmap, Status as SniStatus, StatusNotifierItem};
use system_tray::menu::{MenuItem, MenuType, ToggleState, ToggleType};
use tokio::sync::{broadcast, mpsc, watch};

use super::model::{
    Availability, Check, Pixmap, Tray, TrayCommand, TrayEvent, TrayIcon, TrayItem, TrayItemId,
    TrayMenuEntry, TrayMenuKind, TrayStatus, reduce_tray,
};
use crate::units::Visibility;

const ITEM_PATH: &str = "/StatusNotifierItem";

/// Connect, seed `tray_tx` with whatever is already registered, then apply every event and
/// command until the client's connection drops. Never panics on a missing bus: a failed
/// `Client::new()` leaves the watch at its `Tray::default()` seed (`ServiceError` doc,
/// `error.rs`: "a missing bus... leaves that service's receiver at its default").
pub(super) async fn run(
    tray_tx: watch::Sender<Tray>,
    mut commands: mpsc::UnboundedReceiver<TrayCommand>,
) {
    let client = match Client::new().await {
        Ok(client) => client,
        Err(error) => {
            eprintln!("sill-services: tray: no session bus for the SNI host: {error}");
            return;
        }
    };
    let mut events = client.subscribe();
    let mut state = seed(&client);
    let _ = tray_tx.send(state.clone());

    let mut commands_open = true;
    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(event) => {
                    if let Some(tray_event) = map_event(&client, event) {
                        state = reduce_tray(state, tray_event);
                        let _ = tray_tx.send(state.clone());
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            command = commands.recv(), if commands_open => match command {
                Some(command) => handle_command(&client, command).await,
                None => commands_open = false,
            },
        }
    }
}

/// Every item already on the bus when the client connected, in the order `Client` returned
/// them.
fn seed(client: &Client) -> Tray {
    let mut state = Tray::default();
    let items = client.items();
    let items = items
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (destination, (item, _menu)) in items.iter() {
        state = reduce_tray(
            state,
            TrayEvent::Registered(to_tray_item(destination, item)),
        );
    }
    state
}

/// This item's id: `Client`'s events name it by bus destination alone (module doc).
fn item_id(destination: &str) -> TrayItemId {
    TrayItemId(format!("{destination}{ITEM_PATH}"))
}

/// The bus destination an id names — the inverse of [`item_id`].
fn destination_of(id: &TrayItemId) -> &str {
    id.0.strip_suffix(ITEM_PATH).unwrap_or(&id.0)
}

fn to_tray_item(destination: &str, item: &StatusNotifierItem) -> TrayItem {
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
fn to_tray_icon(name: Option<&str>, pixmaps: Option<&[SniPixmap]>) -> TrayIcon {
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

fn to_tray_status(status: SniStatus) -> TrayStatus {
    match status {
        SniStatus::Passive => TrayStatus::Passive,
        SniStatus::Active | SniStatus::Unknown => TrayStatus::Active,
        SniStatus::NeedsAttention => TrayStatus::NeedsAttention,
    }
}

fn to_check(item: &MenuItem) -> Check {
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

fn to_menu_entry(item: &MenuItem) -> TrayMenuEntry {
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
fn cached_menu(client: &Client, destination: &str) -> Vec<TrayMenuEntry> {
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
fn menu_path_of(client: &Client, id: &TrayItemId) -> Option<String> {
    let destination = destination_of(id);
    let items = client.items();
    let items = items
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    items
        .get(destination)
        .and_then(|(item, _)| item.menu.clone())
}

/// `event` as a [`TrayEvent`], or `None` when it has no home in the frozen model: SNI's
/// `AttentionIcon`, `OverlayIcon` and `Tooltip` updates carry no corresponding `TrayEvent`
/// variant, so they change nothing here (FINDINGS.md "M1 svc-tray").
fn map_event(client: &Client, event: Event) -> Option<TrayEvent> {
    match event {
        Event::Add(destination, item) => {
            Some(TrayEvent::Registered(to_tray_item(&destination, &item)))
        }
        Event::Remove(destination) => Some(TrayEvent::Removed(item_id(&destination))),
        Event::Update(destination, update) => match update {
            UpdateEvent::Title(title) => Some(TrayEvent::Title {
                item: item_id(&destination),
                title: title.unwrap_or_default(),
            }),
            UpdateEvent::Status(status) => Some(TrayEvent::Status {
                item: item_id(&destination),
                status: to_tray_status(status),
            }),
            UpdateEvent::Icon {
                icon_name,
                icon_pixmap,
            } => Some(TrayEvent::Icon {
                item: item_id(&destination),
                icon: to_tray_icon(icon_name.as_deref(), icon_pixmap.as_deref()),
            }),
            UpdateEvent::Menu(_) | UpdateEvent::MenuDiff(_) | UpdateEvent::MenuConnect(_) => {
                Some(TrayEvent::Menu {
                    item: item_id(&destination),
                    menu: cached_menu(client, &destination),
                })
            }
            UpdateEvent::AttentionIcon(_)
            | UpdateEvent::OverlayIcon(_)
            | UpdateEvent::Tooltip(_) => None,
        },
    }
}

/// Send `command` over the bus. Errors are the far end going away mid-call (the app quit, the
/// menu closed) — nothing a shell surface can act on, so they are logged, not propagated: a
/// tray click that lands a beat late is not worth failing the surface over.
///
/// [`TrayCommand`] has no `Scroll` variant (the frozen model, `model.rs`), so
/// `org.kde.StatusNotifierItem.Scroll` is never sent — see FINDINGS.md "M1 svc-tray".
async fn handle_command(client: &Client, command: TrayCommand) {
    let result = match command {
        TrayCommand::Activate { item, x, y } => {
            client
                .activate(ActivateRequest::Default {
                    address: destination_of(&item).to_owned(),
                    x,
                    y,
                })
                .await
        }
        TrayCommand::SecondaryActivate { item, x, y } => {
            client
                .activate(ActivateRequest::Secondary {
                    address: destination_of(&item).to_owned(),
                    x,
                    y,
                })
                .await
        }
        TrayCommand::MenuPick { item, entry } => match menu_path_of(client, &item) {
            Some(menu_path) => {
                client
                    .activate(ActivateRequest::MenuItem {
                        address: destination_of(&item).to_owned(),
                        menu_path,
                        submenu_id: entry,
                    })
                    .await
            }
            None => return,
        },
        TrayCommand::MenuOpening { item } => match menu_path_of(client, &item) {
            Some(menu_path) => client
                .about_to_show_menuitem(destination_of(&item).to_owned(), menu_path, 0)
                .await
                .map(|_needs_update| ()),
            None => return,
        },
    };
    if let Err(error) = result {
        eprintln!("sill-services: tray: command failed: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::{destination_of, item_id, to_check, to_menu_entry, to_tray_icon, to_tray_status};
    use crate::tray::model::{Availability, Check, TrayIcon, TrayItemId, TrayMenuKind, TrayStatus};
    use crate::units::Visibility;
    use system_tray::item::{IconPixmap as SniPixmap, Status as SniStatus};
    use system_tray::menu::{MenuItem, MenuType, ToggleState, ToggleType};

    #[test]
    fn item_id_round_trips_through_destination_of() {
        let id = item_id(":1.42");
        assert_eq!(id, TrayItemId(":1.42/StatusNotifierItem".to_owned()));
        assert_eq!(destination_of(&id), ":1.42");
    }

    #[test]
    fn a_name_wins_over_a_pixmap() {
        let pixmaps = vec![SniPixmap {
            width: 22,
            height: 22,
            pixels: vec![0; 22 * 22 * 4],
        }];
        assert_eq!(
            to_tray_icon(Some("mail-symbolic"), Some(&pixmaps)),
            TrayIcon::Named("mail-symbolic".to_owned())
        );
    }

    #[test]
    fn an_empty_name_falls_back_to_the_largest_pixmap() {
        let pixmaps = vec![
            SniPixmap {
                width: 16,
                height: 16,
                pixels: vec![1; 16 * 16 * 4],
            },
            SniPixmap {
                width: 32,
                height: 32,
                pixels: vec![2; 32 * 32 * 4],
            },
        ];
        let icon = to_tray_icon(Some(""), Some(&pixmaps));
        match icon {
            TrayIcon::Pixmap(pixmap) => {
                assert_eq!((pixmap.width, pixmap.height), (32, 32));
            }
            other => panic!("expected the 32x32 pixmap, got {other:?}"),
        }
    }

    #[test]
    fn neither_name_nor_pixmap_is_none() {
        assert_eq!(to_tray_icon(None, None), TrayIcon::None);
    }

    #[test]
    fn status_maps_unknown_to_active() {
        let cases = [
            (SniStatus::Passive, TrayStatus::Passive),
            (SniStatus::Active, TrayStatus::Active),
            (SniStatus::Unknown, TrayStatus::Active),
            (SniStatus::NeedsAttention, TrayStatus::NeedsAttention),
        ];
        for (input, want) in cases {
            assert_eq!(to_tray_status(input), want, "{input:?}");
        }
    }

    fn menu_item(menu_type: MenuType) -> MenuItem {
        MenuItem {
            id: 3,
            menu_type,
            enabled: true,
            visible: true,
            ..MenuItem::default()
        }
    }

    #[test]
    fn a_separator_carries_no_item_fields() {
        let entry = to_menu_entry(&menu_item(MenuType::Separator));
        assert_eq!(entry.id, 3);
        assert_eq!(entry.kind, TrayMenuKind::Separator);
    }

    #[test]
    fn a_disabled_hidden_item_maps_its_flags() {
        let mut item = menu_item(MenuType::Standard);
        item.label = Some("Quit".to_owned());
        item.enabled = false;
        item.visible = false;
        let entry = to_menu_entry(&item);
        match entry.kind {
            TrayMenuKind::Item {
                label,
                enabled,
                visibility,
                ..
            } => {
                assert_eq!(label, "Quit");
                assert_eq!(enabled, Availability::Disabled);
                assert_eq!(visibility, Visibility::Hidden);
            }
            other => panic!("expected an Item, got {other:?}"),
        }
    }

    #[test]
    fn checkable_items_map_their_toggle_state() {
        let cases = [
            (ToggleType::CannotBeToggled, ToggleState::On, Check::None),
            (ToggleType::Checkmark, ToggleState::On, Check::Checked),
            (ToggleType::Checkmark, ToggleState::Off, Check::Unchecked),
            (
                ToggleType::Radio,
                ToggleState::Indeterminate,
                Check::Unchecked,
            ),
        ];
        for (toggle_type, toggle_state, want) in cases {
            let mut item = menu_item(MenuType::Standard);
            item.toggle_type = toggle_type;
            item.toggle_state = toggle_state;
            assert_eq!(to_check(&item), want, "{toggle_type:?}/{toggle_state:?}");
        }
    }

    #[test]
    fn children_recurse() {
        let mut child = menu_item(MenuType::Standard);
        child.id = 9;
        child.label = Some("Sub".to_owned());
        let mut parent = menu_item(MenuType::Standard);
        parent.label = Some("Parent".to_owned());
        parent.submenu = vec![child];
        let entry = to_menu_entry(&parent);
        match entry.kind {
            TrayMenuKind::Item { children, .. } => {
                assert_eq!(children.len(), 1);
                assert_eq!(children[0].id, 9);
            }
            other => panic!("expected an Item, got {other:?}"),
        }
    }
}
