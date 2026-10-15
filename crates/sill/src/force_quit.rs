//! The daemon's half of the Force Quit Applications panel: the compositor's view of the desktop
//! fills the list on the link (`daemon::events`), the panel surfaces show it, and the app the
//! person confirmed comes back on the link. [`spawn_asks`] follows the confirmations and sends
//! each to the host as a command for the desktop arm.

use shell_host::HostHandle;
use sill_model::desktop::DesktopCommand;
use sill_ui_kit::links::{ForceQuitAsked, Links};
use tokio::runtime::Handle;

use crate::daemon::msg::ShellMsg;

/// The command that ends the app `asked` names.
///
/// ```
/// use sill_model::desktop::DesktopCommand;
/// use sill_ui_kit::links::ForceQuitLink;
/// let link = ForceQuitLink::new();
/// link.ask("term");
/// let asked = link.last_asked().expect("asked");
/// assert_eq!(sill::force_quit::command_of(&asked), DesktopCommand::ForceQuitApp("term".to_owned()));
/// ```
pub fn command_of(asked: &ForceQuitAsked) -> DesktopCommand {
    DesktopCommand::ForceQuitApp(asked.app.clone())
}

/// Follow the confirmations on `links` on `rt` and send each to the host, until the host is gone.
pub fn spawn_asks(rt: &Handle, links: &Links, host: HostHandle<ShellMsg>) {
    let mut asks = links.force_quit.subscribe_asked();
    rt.spawn(async move {
        loop {
            if asks.changed().await.is_err() {
                return;
            }
            let Some(asked) = asks.borrow_and_update().clone() else {
                continue;
            };
            if host.send(ShellMsg::Command(command_of(&asked))).is_err() {
                return;
            }
        }
    });
}
