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

use sill_model::tray::{ScrollAxis, ScrollDelta, Tray, TrayCommand, TrayEvent, TrayItemId, step};
use system_tray::client::{ActivateRequest, Client, Event, UpdateEvent};
use tokio::sync::{broadcast, mpsc, watch};

use super::items::{destination_of, item_id, seed, to_tray_icon, to_tray_item, to_tray_status};
use super::menu::{cached_menu, menu_path_of};
use crate::env::{Bus, BusRoute};

pub(super) const ITEM_PATH: &str = "/StatusNotifierItem";

/// The interface `Scroll` is a method of.
const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";

/// Connect, seed `tray_tx` with whatever is already registered, then apply every event and
/// command until the client's connection drops. Never panics on a missing bus: a failed
/// `Client::new()` leaves the watch at its `Tray::default()` seed (`ServiceError` doc,
/// `error.rs`: "a missing bus... leaves that service's receiver at its default").
pub(crate) async fn run(
    bus: Bus,
    tray_tx: watch::Sender<Tray>,
    mut commands: mpsc::UnboundedReceiver<TrayCommand>,
) {
    match bus.route() {
        BusRoute::Usual => {}
        BusRoute::Absent => return,
        // `Client::new()` reaches only the process's own session bus.
        BusRoute::Address(_) => {
            eprintln!("sill-services: tray: the SNI host runs only on the usual session bus");
            return;
        }
    }
    let client = match Client::new().await {
        Ok(client) => client,
        Err(error) => {
            eprintln!("sill-services: tray: no session bus for the SNI host: {error}");
            return;
        }
    };
    // `Client` keeps its connection private and has no scroll or context-menu call, so `Scroll`
    // and `ContextMenu` go over a second session connection of our own; without one, both are
    // dropped.
    let bus = bus.connect().await.ok().flatten();
    let mut events = client.subscribe();
    let mut state = seed(&client);
    let _ = tray_tx.send(state.clone());

    let mut commands_open = true;
    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(event) => {
                    if let Some(tray_event) = map_event(&client, event) {
                        state = step(state, tray_event);
                        let _ = tray_tx.send(state.clone());
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            command = commands.recv(), if commands_open => match command {
                Some(command) => handle_command(&client, bus.as_ref(), command).await,
                None => commands_open = false,
            },
        }
    }
}

/// `event` as a [`TrayEvent`], or `None` when it has no home in the frozen model: SNI's
/// `AttentionIcon`, `OverlayIcon` and `Tooltip` updates carry no corresponding `TrayEvent`
/// variant, so they change nothing here.
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
/// `Scroll` and `ContextMenu` are sent on `bus`, not through `client` (see [`run`]).
async fn handle_command(client: &Client, bus: Option<&zbus::Connection>, command: TrayCommand) {
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
        TrayCommand::ContextMenu { item, x, y } => match bus {
            Some(bus) => context_menu(bus, &item, x, y).await,
            None => return,
        },
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
        TrayCommand::Scroll { item, delta } => match bus {
            Some(bus) => scroll(bus, &item, delta).await,
            None => return,
        },
    };
    if let Err(error) = result {
        eprintln!("sill-services: tray: command failed: {error}");
    }
}

/// `org.kde.StatusNotifierItem.ContextMenu(x, y)` on `item`: the item pops up its own menu
/// (the SNI spec's answer to a secondary click on an item with no dbusmenu).
async fn context_menu(
    bus: &zbus::Connection,
    item: &TrayItemId,
    x: i32,
    y: i32,
) -> Result<(), system_tray::error::Error> {
    bus.call_method(
        Some(destination_of(item)),
        ITEM_PATH,
        Some(ITEM_INTERFACE),
        "ContextMenu",
        &(x, y),
    )
    .await
    .map(|_reply| ())
    .map_err(system_tray::error::Error::from)
}

/// `org.kde.StatusNotifierItem.Scroll(delta, orientation)` on `item`.
async fn scroll(
    bus: &zbus::Connection,
    item: &TrayItemId,
    delta: ScrollDelta,
) -> Result<(), system_tray::error::Error> {
    bus.call_method(
        Some(destination_of(item)),
        ITEM_PATH,
        Some(ITEM_INTERFACE),
        "Scroll",
        &scroll_args(delta),
    )
    .await
    .map(|_reply| ())
    .map_err(system_tray::error::Error::from)
}

/// SNI `Scroll`'s two arguments: the delta and the orientation word the spec names.
fn scroll_args(delta: ScrollDelta) -> (i32, &'static str) {
    let orientation = match delta.axis {
        ScrollAxis::Vertical => "vertical",
        ScrollAxis::Horizontal => "horizontal",
    };
    (delta.steps, orientation)
}

#[cfg(test)]
mod tests {
    use super::super::menu::{to_check, to_menu_entry, to_menu_icon};
    use super::{destination_of, item_id, scroll_args, to_tray_icon, to_tray_status};
    use sill_model::tray::{
        Check, ScrollAxis, ScrollDelta, TrayIcon, TrayItemId, TrayMenuIcon, TrayMenuKind,
        TrayStatus,
    };
    use sill_model::units::{Availability, Visibility};
    use system_tray::item::{IconPixmap as SniPixmap, Status as SniStatus};
    use system_tray::menu::{MenuItem, MenuType, ToggleState, ToggleType};

    #[test]
    fn a_scroll_becomes_sni_arguments() {
        let cases = [
            (ScrollAxis::Vertical, 1, (1, "vertical")),
            (ScrollAxis::Vertical, -3, (-3, "vertical")),
            (ScrollAxis::Horizontal, 2, (2, "horizontal")),
        ];
        for (axis, steps, want) in cases {
            assert_eq!(
                scroll_args(ScrollDelta { axis, steps }),
                want,
                "{axis:?} {steps}"
            );
        }
    }

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
            (ToggleType::Radio, ToggleState::On, Check::Checked),
            (ToggleType::Radio, ToggleState::Off, Check::Unchecked),
            (ToggleType::Radio, ToggleState::Indeterminate, Check::Mixed),
            (
                ToggleType::Checkmark,
                ToggleState::Indeterminate,
                Check::Mixed,
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

    #[test]
    fn a_row_keeps_its_icon_and_loses_its_access_key() {
        let cases = [
            (None, None, TrayMenuIcon::None),
            (Some(""), Some(vec![]), TrayMenuIcon::None),
            (Some("mail"), None, TrayMenuIcon::Named("mail".to_owned())),
            (None, Some(vec![1, 2]), TrayMenuIcon::Png(vec![1, 2])),
            (Some("mail"), Some(vec![1]), TrayMenuIcon::Png(vec![1])),
        ];
        for (name, data, want) in cases {
            let mut item = menu_item(MenuType::Standard);
            item.icon_name = name.map(str::to_owned);
            item.icon_data = data.clone();
            assert_eq!(to_menu_icon(&item), want, "{name:?} {data:?}");
        }
        let mut item = menu_item(MenuType::Standard);
        item.label = Some("_Quit".to_owned());
        match to_menu_entry(&item).kind {
            TrayMenuKind::Item { label, .. } => assert_eq!(label, "Quit"),
            other => panic!("expected an Item, got {other:?}"),
        }
    }
}
