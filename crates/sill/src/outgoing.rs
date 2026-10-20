//! What the shell's surfaces send back for the desktop to carry out: a window menu pick, a Split
//! View answer, the display trial's answer, a confirmed force quit, a click in the Spaces bar.
//! Each rides an `Outbox` on its link; one forwarder, [`forward`], hands every item to the host
//! as the commands it means. A link that sends is registered once, by implementing
//! [`Outgoing`], and listed in [`spawn_all`].

use shell_host::HostHandle;
use sill_model::desktop::DesktopCommand;
use sill_ui_kit::links::{Links, Sent};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;

/// A link whose surfaces send items for the desktop to carry out.
pub trait Outgoing: 'static {
    /// What the surfaces send.
    type Item: Clone + Send + Sync + 'static;

    /// A receiver for the latest send.
    fn follow(&self) -> watch::Receiver<Option<Sent<Self::Item>>>;

    /// The commands an item is.
    fn commands(sent: &Sent<Self::Item>) -> Vec<DesktopCommand>;
}

/// Follow `link`'s sends on `rt` and send each one's commands to the host, until the host is gone.
pub fn forward<L: Outgoing>(rt: &Handle, link: &L, host: HostHandle<ShellMsg>) {
    let mut sends = link.follow();
    rt.spawn(async move {
        loop {
            if sends.changed().await.is_err() {
                return;
            }
            let Some(sent) = sends.borrow_and_update().clone() else {
                continue;
            };
            for command in L::commands(&sent) {
                if host.send(ShellMsg::Command(command)).is_err() {
                    return;
                }
            }
        }
    });
}

/// Forward every link of `links` that sends.
pub fn spawn_all(rt: &Handle, links: &Links, host: &HostHandle<ShellMsg>) {
    forward(rt, &links.window_menu, host.clone());
    forward(rt, &links.split_picker, host.clone());
    forward(rt, &links.display_trial, host.clone());
    forward(rt, &links.force_quit, host.clone());
    forward(rt, &links.spaces_bar, host.clone());
}
