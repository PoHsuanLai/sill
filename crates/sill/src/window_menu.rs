//! The daemon's half of a window's menu: the compositor's note opens it on the link
//! ([`crate::daemon::compositor::apply`]), the window menu surfaces show it, and an entry the
//! person picked comes back on the link. [`spawn_picks`] follows the picks and sends each to the
//! host as a command for the desktop arm; a dismissal wrote nothing, so nothing is sent.

use shell_host::HostHandle;
use sill_model::desktop::DesktopCommand;
use sill_ui_kit::links::{Links, WindowMenuPicked};
use tokio::runtime::Handle;

use crate::daemon::msg::ShellMsg;

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

/// Follow the picks on `links` on `rt` and send each to the host, until the host is gone.
pub fn spawn_picks(rt: &Handle, links: &Links, host: HostHandle<ShellMsg>) {
    let mut picks = links.window_menu.subscribe_picks();
    rt.spawn(async move {
        loop {
            if picks.changed().await.is_err() {
                return;
            }
            let Some(picked) = picks.borrow_and_update().clone() else {
                continue;
            };
            if host.send(ShellMsg::Command(command_of(&picked))).is_err() {
                return;
            }
        }
    });
}
