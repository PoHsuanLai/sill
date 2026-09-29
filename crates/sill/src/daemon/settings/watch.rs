//! The settings file's watch: each settled change becomes a `ShellMsg::Settings` for the host
//! thread, which publishes it.

use std::path::PathBuf;

use shell_host::HostHandle;
use tokio::runtime::Runtime;

use crate::daemon::msg::ShellMsg;

/// Watch `dir`'s `settings.toml` on `runtime`, sending each settled file to `host`. A directory
/// that cannot be watched is logged and the settings then change only through `sill reload`.
pub fn spawn_watch(runtime: &Runtime, dir: PathBuf, host: HostHandle<ShellMsg>) {
    // `sill_settings::watch` calls `tokio::spawn` in its own body, which needs an entered
    // runtime on the calling thread; `host.run` does not enter it until later.
    let _entered = runtime.enter();
    match sill_settings::watch(&dir) {
        Ok(mut watch) => {
            runtime.spawn(async move {
                while let Some(file) = watch.changed().await {
                    if host.send(ShellMsg::Settings(Box::new(file))).is_err() {
                        break;
                    }
                }
            });
        }
        Err(error) => eprintln!("sill: watching {}: {error}", dir.display()),
    }
}
