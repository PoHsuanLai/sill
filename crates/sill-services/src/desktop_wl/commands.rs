//! The command channel between the service's `Control`s and the thread's loop, shared by both
//! arms of the windows service.

use sill_model::windows::WlCommand;
use smithay_client_toolkit::reexports::calloop::channel;
use tokio::sync::mpsc::UnboundedReceiver;

/// Hand every command to the loop; once the loop has stopped, drop them.
pub(crate) fn forward_commands(
    mut commands: UnboundedReceiver<WlCommand>,
    to_loop: channel::Sender<WlCommand>,
) {
    while let Some(command) = commands.blocking_recv() {
        let _dropped_after_the_loop_stopped = to_loop.send(command);
    }
}

/// Sleep on the command channel, dropping commands, until every `Control` is gone.
pub(crate) fn park_commands(mut commands: UnboundedReceiver<WlCommand>) {
    while commands.blocking_recv().is_some() {}
}
