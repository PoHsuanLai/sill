//! The focused app's menus as the bar draws them and an agent reads them: an app that serves
//! `com.canonical.dbusmenu` at `org.quire.AppMenu.<app_id>` (quire design/27 §5.2 rule a), what
//! the service follows of it, the titles the bar makes from it, and what activating an item
//! means for the gate.
//!
//! Everything an app sends (labels, titles, `x-quire-command-id`) is a claim, so [`route`] never
//! lets one lower an item's effect unless the app is proven to be the app whose manifest names
//! the command.

mod command;
mod follow;
mod layout;
mod listing;
mod model;
mod own;
mod route;
mod step;
mod titles;
mod window;

#[cfg(test)]
mod tests;

pub use command::{AppMenuCommand, AppMenuEvent};
pub use follow::{Followers, Following};
pub use layout::{Node, NodeProps, NodeType, ToggleKind, rows_of};
pub use listing::{LISTING_MAX, listing};
pub use model::{AppMenus, Follower, MenuApp, MenuName, ServedMenu};
pub use own::{OwnGroup, OwnItem, OwnRow, QuitRow, commands as own_commands, own_groups};
pub use route::{ActionIndex, Identity, ItemKey, Route, effect_of, route_of};
pub use step::step;
pub use titles::{TitledMenu, Titles, titles};
pub use window::window_of;
