//! The settings file's watch: each settled change becomes a `ShellMsg::Settings` for the host
//! thread, which publishes it.

use ds_blitz::TokioSpawner;
use ds_settings::{Store, WatchState};
use shell_host::HostHandle;
use sill_settings::ShellFile;
use tokio::runtime::Runtime;

use super::store::report;
use crate::daemon::msg::ShellMsg;

/// Watch `store`'s `settings.toml` on `runtime`, sending each settled file to `host`. A directory
/// that cannot be watched is logged and the settings then change only through `sill reload`.
pub fn spawn_watch(runtime: &Runtime, store: &Store, host: HostHandle<ShellMsg>) {
    let mut watch = store.watch::<ShellFile>(&TokioSpawner::on(runtime.handle().clone()));
    if let WatchState::Blind { reason } = watch.state() {
        eprintln!("sill: watching the settings: {reason}");
        return;
    }
    runtime.spawn(async move {
        while let Some(loaded) = watch.changed().await {
            report(&loaded);
            if host
                .send(ShellMsg::Settings(Box::new(loaded.value)))
                .is_err()
            {
                break;
            }
        }
    });
}
