//! The person's `style.css`: read at start, watched, and published to the feed every surface
//! root draws it from (ARCHITECTURE.md section 8). It takes the settings file's path: the watch
//! sends each settled text to the host thread, which publishes it; `sill reload` re-reads it.
//!
//! A missing file is an empty style; text that is not UTF-8 keeps the last good text and is
//! reported on stderr like any damaged settings file. Nothing here parses the CSS:
//! `sill style check` reports what is wrong in it.

use ds_blitz::TokioSpawner;
use ds_settings::{Store, UserStyle, WatchState};
use shell_host::{HostEvent, HostHandle};
use tokio::runtime::Runtime;
use tokio::sync::watch;

use super::store::report;
use crate::daemon::msg::ShellMsg;

/// The stored stylesheet, empty when there is none; what could not be read is reported.
pub fn load_user_style(store: &Store) -> UserStyle {
    let loaded = store.load::<UserStyle>();
    report(&loaded);
    loaded.value
}

/// Re-read `style.css` from `store` and publish it.
pub fn reload_user_style(store: &Store, user_style: &watch::Sender<UserStyle>) {
    user_style.send_replace(load_user_style(store));
}

/// Watch `store`'s `style.css` on `runtime`, sending each settled text to `host`. A directory
/// that cannot be watched is logged and the style then changes only through `sill reload`.
pub fn spawn_user_style_watch(runtime: &Runtime, store: &Store, host: HostHandle<ShellMsg>) {
    let mut watch = store.watch::<UserStyle>(&TokioSpawner::on(runtime.handle().clone()));
    if let WatchState::Blind { reason } = watch.state() {
        eprintln!("sill: watching the style: {reason}");
        return;
    }
    runtime.spawn(async move {
        while let Some(loaded) = watch.changed().await {
            report(&loaded);
            if host
                .send(ShellMsg::UserStyle(Box::new(loaded.value)))
                .is_err()
            {
                break;
            }
        }
    });
}

/// `event`'s text, published to `user_style`, if it is a watched change (`ShellMsg::UserStyle`);
/// anything else is a no-op.
pub fn publish_watched_style(user_style: &watch::Sender<UserStyle>, event: &HostEvent<ShellMsg>) {
    if let HostEvent::App(ShellMsg::UserStyle(text)) = event {
        user_style.send_replace((**text).clone());
    }
}

#[cfg(test)]
#[path = "user_style_tests.rs"]
mod tests;
