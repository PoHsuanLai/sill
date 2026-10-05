//! Reading an app's menu on demand.

use sill_model::app_menu::{MenuApp, MenuName, ServedMenu, QuitRow, listing, own_groups, rows_of};
use zbus::fdo::DBusProxy;
use zbus::names::{BusName, OwnedUniqueName};

use super::{MenuAgent, MenuFault};
use crate::app_menu::wire::read_layout;
use docket_client::Transport;

/// The menus of one app, as text for the planner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuListed {
    /// The app's Wayland `app_id`.
    pub app: MenuApp,
    /// Its menus, one row a line (`sill_model::app_menu::listing`). The app wrote the words.
    pub text: String,
}

/// An app's menu and the process that serves it.
#[derive(Debug)]
pub(super) struct Served {
    pub owner: OwnedUniqueName,
    pub menu: ServedMenu,
}

impl<T: Transport> MenuAgent<T> {
    /// The menus of the window asked for (the focused one when `window` is `None`): the app's own
    /// when it serves them, and the shell's rows. An app with no menu has the shell's rows alone.
    pub async fn list(&self, window: Option<&str>) -> Result<MenuListed, MenuFault> {
        let target = self.target(window)?;
        let served = self.serve(&target.app).await?;
        let menu = served.as_ref().map(|served| &served.menu);
        let windows: Vec<_> = target
            .toplevels
            .windows
            .iter()
            .filter(|w| w.app_id == target.app.0)
            .map(|w| (w.key, w.title.clone()))
            .collect();
        let own = own_groups(&target.app.0, &windows, QuitRow::of(menu));
        Ok(MenuListed {
            text: listing(&target.app.0, &own, menu),
            app: target.app,
        })
    }

    /// The app's menu as its current owner serves it, or `None` when nobody owns the name.
    pub(super) async fn serve(&self, app: &MenuApp) -> Result<Option<Served>, MenuFault> {
        let Some(name) = (self.links.name_of)(app) else {
            return Ok(None);
        };
        let Some(owner) = self.owner_of(&name).await? else {
            return Ok(None);
        };
        let nodes = read_layout(&self.connection, owner.as_str())
            .await
            .map_err(|_| MenuFault::Failed("the app's menu could not be read".to_owned()))?;
        Ok(Some(Served {
            owner,
            menu: ServedMenu {
                name,
                menus: rows_of(nodes),
            },
        }))
    }

    async fn owner_of(&self, name: &MenuName) -> Result<Option<OwnedUniqueName>, MenuFault> {
        let failed = |_: zbus::Error| MenuFault::Failed("the bus could not be asked".to_owned());
        let dbus = DBusProxy::new(&self.connection).await.map_err(failed)?;
        let name = BusName::try_from(name.0.as_str())
            .map_err(|_| MenuFault::Failed("the menu name is not a bus name".to_owned()))?;
        let owned = dbus.name_has_owner(name.clone()).await;
        if !owned.map_err(|e| failed(e.into()))? {
            return Ok(None);
        }
        Ok(dbus.get_name_owner(name).await.ok())
    }
}
