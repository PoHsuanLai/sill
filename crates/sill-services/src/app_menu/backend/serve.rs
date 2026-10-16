//! One app's menu, followed while its name has an owner that is the app: read at once, read again
//! whenever the app announces a change, and forgotten when the owner goes (design/27 §5.2 rule c).
//! An owner that is not the app (a program that only claimed the name) is treated as no owner.

use std::sync::Arc;

use sill_model::app_menu::{
    AppMenuEvent, MenuApp, MenuClient, MenuName, MenuTrust, ServedMenu, rows_of,
};
use tokio::sync::mpsc::UnboundedSender;
use zbus::{Connection, Proxy};

use crate::app_menu::wire::{MENU_INTERFACE, MENU_PATH, read_layout};
use crate::callers::{Peers, menu_shown};
use crate::owner;
use crate::stream::next;

/// Follow `app`'s menu at `name` until the receiver of `events` goes.
pub(super) async fn follow(
    connection: Connection,
    peers: Arc<Peers>,
    app: MenuApp,
    name: MenuName,
    events: UnboundedSender<AppMenuEvent>,
) {
    loop {
        owner::appeared(&connection, &name.0).await;
        tokio::select! {
            () = session(&connection, &peers, &app, &name, &events) => {
                // The app answered badly: nothing to show until its owner changes.
                owner::changed(&connection, &name.0).await;
            }
            () = owner::changed(&connection, &name.0) => {}
        }
        if events.send(AppMenuEvent::Gone(app.clone())).is_err() {
            return;
        }
    }
}

/// Read the menu of the name's owner, if it is the app, then read it again after each
/// announcement, until a read fails or `events` is closed. The menu is read from the owner's own
/// unique name, so a program that takes the name over after the check is not read.
async fn session(
    connection: &Connection,
    peers: &Peers,
    app: &MenuApp,
    name: &MenuName,
    events: &UnboundedSender<AppMenuEvent>,
) {
    let Some(owner) = owner::current(connection, &name.0).await else {
        return;
    };
    let client = MenuClient::unreported(app);
    if menu_shown(peers, &owner, &client).await == MenuTrust::Hidden {
        return;
    }
    let Ok(proxy) = Proxy::new(connection, owner.as_str(), MENU_PATH, MENU_INTERFACE).await else {
        return;
    };
    // Both streams are open before the first read, so no announcement falls between.
    let (Ok(mut layout), Ok(mut properties)) = (
        proxy.receive_signal("LayoutUpdated").await,
        proxy.receive_signal("ItemsPropertiesUpdated").await,
    ) else {
        return;
    };
    loop {
        let Ok(nodes) = read_layout(connection, owner.as_str()).await else {
            return;
        };
        let menu = ServedMenu {
            name: name.clone(),
            menus: rows_of(nodes),
        };
        let sent = events.send(AppMenuEvent::Served {
            app: app.clone(),
            menu,
        });
        if sent.is_err() {
            return;
        }
        tokio::select! {
            heard = next(&mut layout) => if heard.is_none() { return },
            heard = next(&mut properties) => if heard.is_none() { return },
        }
    }
}
