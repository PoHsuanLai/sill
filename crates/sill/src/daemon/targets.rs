//! The minimise targets, on the daemon's side: what the docks and the desktop say reaches the
//! host thread as messages, where [`ShellSeat`] keeps the compositor's targets equal to the
//! docks' icons (`sill_model::desktop::target`). Nothing runs on a session without the desktop's
//! own compositor.

use shell_host::HostHandle;
use tokio::runtime::Handle;
use tokio::sync::watch;

use super::msg::ShellMsg;

/// Send `message` now and after each change of `watched`, until the host or the sender is gone.
/// The value is not carried: whoever handles the message reads the receiver it holds.
pub fn nudge<T>(
    runtime: &Handle,
    host: HostHandle<ShellMsg>,
    mut watched: watch::Receiver<T>,
    message: fn() -> ShellMsg,
) where
    T: Send + Sync + 'static,
{
    runtime.spawn(async move {
        loop {
            if host.send(message()).is_err() || watched.changed().await.is_err() {
                return;
            }
        }
    });
}

/// Send `watched`'s value now and after each change as `wrap` of it, until the host or the
/// sender is gone.
pub fn forward<T>(
    runtime: &Handle,
    host: HostHandle<ShellMsg>,
    mut watched: watch::Receiver<T>,
    wrap: fn(T) -> ShellMsg,
) where
    T: Clone + Send + Sync + 'static,
{
    runtime.spawn(async move {
        loop {
            let value = watched.borrow_and_update().clone();
            if host.send(wrap(value)).is_err() || watched.changed().await.is_err() {
                return;
            }
        }
    });
}
