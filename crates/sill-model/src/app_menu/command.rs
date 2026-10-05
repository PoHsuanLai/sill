//! What the app-menu service is told and what it hears.

use super::{MenuApp, ServedMenu};

/// What the bar tells the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMenuCommand {
    /// A bar's focused app is now `app` (`None`: none). The service follows the menu of every
    /// app some bar names, and stops following one nobody names.
    Follow {
        by: super::Follower,
        app: Option<MenuApp>,
    },
    /// The person picked a row: dbusmenu `Event(id, "clicked")`.
    Pick { app: MenuApp, id: i32 },
    /// A menu is about to show: dbusmenu `AboutToShow(id)`, so the app can refresh it.
    Opening { app: MenuApp, id: i32 },
}

/// One change the service heard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMenuEvent {
    /// `app`'s menu was read (first, or after the app announced a change).
    Served { app: MenuApp, menu: ServedMenu },
    /// `app`'s name lost its owner, or nobody follows `app` any more.
    Gone(MenuApp),
}
