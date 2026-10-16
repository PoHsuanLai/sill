//! The daemon's half of the compositor's permission question: what the person answers on the
//! sheet reaches the compositor as a command for the desktop arm. The capture service holds the
//! question and its sheet (`sill_services::capture_pick`); the arm lives on the host's thread, so
//! [`spawn`] carries each answer there.

use shell_host::HostHandle;
use sill_model::desktop::DesktopCommand;
use tokio::runtime::Handle;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::daemon::msg::ShellMsg;

/// Send each answer the capture service hands over to the host, until the host or the service is
/// gone.
pub fn spawn(
    rt: &Handle,
    mut answers: UnboundedReceiver<DesktopCommand>,
    host: HostHandle<ShellMsg>,
) {
    rt.spawn(async move {
        while let Some(answer) = answers.recv().await {
            if host.send(ShellMsg::Command(answer)).is_err() {
                return;
            }
        }
    });
}
