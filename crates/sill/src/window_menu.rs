//! The daemon's half of a window's menu: the compositor's note opens it on the link
//! ([`crate::daemon::compositor::apply`]), the window menu surfaces show it, and an entry the
//! person picked comes back on the link. [`Outgoing`] sends each pick to the host as a
//! command for the desktop arm; a dismissal wrote nothing, so nothing is sent.

use sill_model::desktop::DesktopCommand;
use sill_ui_kit::links::{WindowMenuLink, WindowMenuPick, WindowMenuPicked};
use tokio::sync::watch;

use crate::outgoing::Outgoing;

/// The command a pick is: the window's menu entry, quoted by the window's identifier.
///
/// ```
/// use sill::window_menu::command_of;
/// use sill_model::desktop::{DesktopCommand, WindowEntry};
/// use sill_ui_kit::links::WindowMenuLink;
///
/// let link = WindowMenuLink::new();
/// link.pick("w3", WindowEntry::Left);
/// let picked = link.last_pick().expect("picked");
/// assert_eq!(
///     command_of(&picked),
///     DesktopCommand::PickWindowMenu("w3".to_owned(), WindowEntry::Left)
/// );
/// ```
pub fn command_of(picked: &WindowMenuPicked) -> DesktopCommand {
    DesktopCommand::PickWindowMenu(picked.window.clone(), picked.entry)
}

impl Outgoing for WindowMenuLink {
    type Item = WindowMenuPick;

    fn follow(&self) -> watch::Receiver<Option<WindowMenuPicked>> {
        self.subscribe_picks()
    }

    fn commands(picked: &WindowMenuPicked) -> Vec<DesktopCommand> {
        vec![command_of(picked)]
    }
}
