//! What the app-menu service publishes: the menus of the apps some bar is following.

use std::collections::BTreeMap;

use crate::tray::TrayMenuEntry;

/// An app's Wayland `app_id`, which the bar reads off the focused toplevel and which decides the
/// bus name its menu is served at.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MenuApp(pub String);

/// The well-known bus name an app's menu is served at (`org.quire.AppMenu.<app_id>`), derived
/// from its [`MenuApp`] by quire's `AppMenuAddress` and nowhere else.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MenuName(pub String);

/// Who asked for an app's menu to be followed: one bar, by its output's name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Follower(pub String);

/// One app's exported menu: its menus, in the bar's order, as the rows the app sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedMenu {
    /// The bus name it is served at.
    pub name: MenuName,
    /// The bar's menus, each a row with its own rows below it.
    pub menus: Vec<TrayMenuEntry>,
}

/// The menus of every followed app that has an owner for its name; an app with none has no entry
/// (design/27 §5.2 rule c).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppMenus {
    pub served: BTreeMap<MenuApp, ServedMenu>,
}
