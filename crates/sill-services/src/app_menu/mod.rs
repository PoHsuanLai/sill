//! The menus of the apps the bar follows. An app that serves `com.canonical.dbusmenu` at
//! `org.quire.AppMenu.<app_id>` (quire design/27 §5.2 rule a) has its menu read, followed
//! (`LayoutUpdated`, `ItemsPropertiesUpdated`) and published; a click, and `AboutToShow` when a
//! menu opens, go back to the app. An app with no owner for its name has no entry, so the bar
//! keeps its own App menu alone (rule c).
//!
//! The service follows an app only while some bar names it ([`AppMenuCommand::Follow`]). The bus
//! name is made by `AppMenuDeps::name_of`, which the daemon fills with quire's `AppMenuAddress`
//! (a services crate links no design system). The agent's two actions, `shell.menu.list` and
//! `shell.menu.activate`, are [`MenuAgent`]'s: they read the menu fresh and never use this
//! service's state.

mod agent;
mod backend;
mod wire;

use sill_model::app_menu::{AppMenuCommand, AppMenus, MenuApp, MenuName};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub use agent::{Activated, Activation, MenuAgent, MenuFault, MenuLinks, MenuListed};

/// The bus name of `app`'s menu, or `None` for an id no name can be made from.
pub type MenuNameOf = fn(&MenuApp) -> Option<MenuName>;

/// What the service needs: how an app's menu name is made.
#[derive(Debug, Clone, Copy)]
pub struct AppMenuDeps {
    pub name_of: MenuNameOf,
}

/// The followed apps' menus as a [`Service`]. Without a session bus it publishes nothing.
#[derive(Debug)]
pub enum AppMenuService {}

impl Service for AppMenuService {
    const NAME: &'static str = "app_menu";
    type State = AppMenus;
    type Command = AppMenuCommand;
    type Deps = AppMenuDeps;

    fn run(
        env: &Env,
        deps: AppMenuDeps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        async move {
            backend::run(bus, deps, io).await;
            Ok(())
        }
    }
}
