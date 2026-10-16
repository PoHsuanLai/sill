//! What `shell.menu.list` and `shell.menu.activate` do, for the shell's provider.
//!
//! The menu is read fresh from the app on every call (nothing here uses the bar's followed
//! state), and everything the app sent is a claim. [`MenuAgent::activate`] sends a dbusmenu click
//! only for an item the table calls a click, and does a verified app's own action through
//! intentd instead, as the shell, so the gate, confirmation, undo journal and audit apply as for
//! a typed call. The effect table itself is `sill_model::app_menu::route_of`.

mod activate;
mod identity;
mod read;

use std::path::PathBuf;
use std::sync::Arc;

use docket_client::{DbusTransport, Intents, Transport};
use sill_model::app_menu::MenuApp;
use sill_model::windows::{Toplevels, WindowModel, WlCommand};
use tokio::sync::watch;
use zbus::Connection;

use super::MenuNameOf;
use crate::callers::Peers;
use crate::control::Control;

pub use activate::{Activated, Activation, MenuFault};
pub use read::MenuListed;

/// What the agent's actions need from the shell besides the bus.
#[derive(Debug, Clone)]
pub struct MenuLinks {
    /// The compositor's windows, which say what is focused.
    pub toplevels: watch::Receiver<WindowModel>,
    /// Where the shell's own rows (Hide, Quit, raising a window) go.
    pub wl: Control<WlCommand>,
    /// How an app's menu name is made.
    pub name_of: MenuNameOf,
}

/// The two actions over one connection to the session bus.
#[derive(Debug)]
pub struct MenuAgent<T: Transport> {
    connection: Connection,
    links: MenuLinks,
    proc_root: PathBuf,
    intents: Intents<T>,
    /// Tells the owner of an app's menu name apart from a program that claimed it.
    peers: Arc<Peers>,
}

impl MenuAgent<DbusTransport> {
    /// The agent speaking to intentd on `connection`, which is the one that owns
    /// `org.quire.Shell`: intentd names the caller's role from the names it owns. `proc_root` is
    /// where a process's cgroup is read to prove who owns a menu's name, and `peers` says whether
    /// the owner is the app the window belongs to.
    pub fn over(
        connection: Connection,
        links: MenuLinks,
        proc_root: PathBuf,
        peers: Arc<Peers>,
    ) -> Self {
        let intents = Intents::over(DbusTransport::new(connection.clone()));
        MenuAgent::with_intents(connection, links, proc_root, intents, peers)
    }
}

impl<T: Transport> MenuAgent<T> {
    /// The agent calling intentd through `intents`.
    pub fn with_intents(
        connection: Connection,
        links: MenuLinks,
        proc_root: PathBuf,
        intents: Intents<T>,
        peers: Arc<Peers>,
    ) -> Self {
        MenuAgent {
            connection,
            links,
            proc_root,
            intents,
            peers,
        }
    }

    /// The windows now, and the app the one asked for (or the focused one) is.
    fn target(&self, window: Option<&str>) -> Result<Target, MenuFault> {
        let toplevels = self.links.toplevels.borrow().toplevels.clone();
        let app = sill_model::app_menu::window_of(&toplevels, window)
            .map(|w| MenuApp(w.app_id.clone()))
            .ok_or(MenuFault::NoWindow)?;
        Ok(Target { app, toplevels })
    }
}

/// The app an ask is about, and every window as it was when asked.
#[derive(Debug)]
struct Target {
    app: MenuApp,
    toplevels: Toplevels,
}
