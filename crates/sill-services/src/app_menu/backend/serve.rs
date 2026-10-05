//! One app's menu, followed while its name has an owner: read at once, read again whenever the
//! app announces a change, and forgotten when the owner goes (design/27 §5.2 rule c).

use sill_model::app_menu::{AppMenuEvent, MenuApp, MenuName, ServedMenu, rows_of};
use tokio::sync::mpsc::UnboundedSender;
use zbus::{Connection, Proxy};

use crate::app_menu::wire::{MENU_INTERFACE, MENU_PATH, read_layout};
use crate::owner;
use crate::stream::next;

/// Follow `app`'s menu at `name` until the receiver of `events` goes.
pub(super) async fn follow(
    connection: Connection,
    app: MenuApp,
    name: MenuName,
    events: UnboundedSender<AppMenuEvent>,
) {
    loop {
        owner::appeared(&connection, &name.0).await;
        tokio::select! {
            () = session(&connection, &app, &name, &events) => {
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

/// Read the menu, then read it again after each announcement, until a read fails or `events` is
/// closed.
async fn session(
    connection: &Connection,
    app: &MenuApp,
    name: &MenuName,
    events: &UnboundedSender<AppMenuEvent>,
) {
    let Ok(proxy) = Proxy::new(connection, name.0.as_str(), MENU_PATH, MENU_INTERFACE).await
    else {
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
        let Ok(nodes) = read_layout(connection, &name.0).await else {
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
