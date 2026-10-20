//! The service's loop: the bars' wishes in, one followed session per wanted app, the menus out.

use std::collections::HashMap;
use std::sync::Arc;

use sill_model::app_menu::{AppMenuCommand, AppMenuEvent, Followers, MenuApp, MenuName, step};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};
use tokio::task::JoinHandle;
use zbus::Connection;

use super::serve;
use crate::app_menu::{AppMenuDeps, AppMenuService, wire};
use crate::callers::CallerSource;
use crate::env::{Bus, ConnectError};
use crate::service::ServiceIo;

/// How many clicks and "about to show" calls are on the way at once: each waits on an app that may
/// be hung, so past this a call is dropped, not queued behind them.
const TELLS_IN_FLIGHT_MAX: usize = 8;

/// A followed app: where its menu is served, and the task that follows it.
struct Followed {
    name: MenuName,
    task: JoinHandle<()>,
}

pub(crate) async fn run(
    bus: Bus,
    callers: Arc<dyn CallerSource>,
    deps: AppMenuDeps,
    io: ServiceIo<AppMenuService>,
) {
    let connection = match bus.try_connect().await {
        Ok(connection) => connection,
        Err(ConnectError::NoBus(_)) => return,
        Err(error) => {
            eprintln!("sill-services: app_menu: no session bus: {error}");
            return;
        }
    };
    let peers = callers.peers_on(connection.clone());
    let ServiceIo {
        state,
        mut commands,
    } = io;
    let (events_tx, mut events) = mpsc::unbounded_channel::<AppMenuEvent>();
    let mut followers = Followers::default();
    let mut followed: HashMap<MenuApp, Followed> = HashMap::new();
    let tells = Arc::new(Semaphore::new(TELLS_IN_FLIGHT_MAX));
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(AppMenuCommand::Follow { by, app }) => {
                    let (next, change) = followers.set(by, app);
                    followers = next;
                    for app in change.stop {
                        if let Some(gone) = followed.remove(&app) {
                            gone.task.abort();
                            let _ = events_tx.send(AppMenuEvent::Gone(app));
                        }
                    }
                    for app in change.start {
                        if let Some(name) = (deps.name_of)(&app) {
                            let task = tokio::spawn(serve::follow(
                                connection.clone(),
                                peers.clone(),
                                deps.windows.clone(),
                                app.clone(),
                                name.clone(),
                                events_tx.clone(),
                            ));
                            followed.insert(app, Followed { name, task });
                        }
                    }
                }
                Some(AppMenuCommand::Pick { app, id }) => {
                    if let Some(at) = followed.get(&app) {
                        tell(&connection, &tells, at.name.clone(), id, Tell::Click);
                    }
                }
                Some(AppMenuCommand::Opening { app, id }) => {
                    if let Some(at) = followed.get(&app) {
                        tell(&connection, &tells, at.name.clone(), id, Tell::AboutToShow);
                    }
                }
                None => break,
            },
            Some(event) = events.recv() => {
                // A read that was in flight when the app stopped being followed.
                if matches!(&event, AppMenuEvent::Served { app, .. } if !followed.contains_key(app)) {
                    continue;
                }
                state.send_if_modified(|menus| {
                    let next = step(menus.clone(), event);
                    let changed = next != *menus;
                    *menus = next;
                    changed
                });
            }
        }
    }
    for at in followed.into_values() {
        at.task.abort();
    }
}

/// What a bar tells an app's menu.
enum Tell {
    Click,
    AboutToShow,
}

/// Send `tell` for row `id` to the app serving `name`, without holding the loop: an app that
/// hangs only costs its own call (for [`crate::deadline::MENU_CALL`]), one that is gone has
/// nothing left to tell, and past [`TELLS_IN_FLIGHT_MAX`] calls on the way it is dropped.
fn tell(connection: &Connection, tells: &Arc<Semaphore>, name: MenuName, id: i32, tell: Tell) {
    let Ok(permit) = Arc::clone(tells).try_acquire_owned() else {
        eprintln!(
            "sill-services: app_menu: {}: too many calls on the way",
            name.0
        );
        return;
    };
    let connection = connection.clone();
    tokio::spawn(async move {
        let _permit: OwnedSemaphorePermit = permit;
        let sent = match tell {
            Tell::Click => wire::click(&connection, &name.0, id).await,
            Tell::AboutToShow => wire::about_to_show(&connection, &name.0, id).await,
        };
        if let Err(error) = sent {
            eprintln!("sill-services: app_menu: {}: {error}", name.0);
        }
    });
}
