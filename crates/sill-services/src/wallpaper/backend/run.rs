//! The wallpaper task: prepare each output's images on the blocking pool, follow the source,
//! publish cached paths.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sill_model::ids::OutputName;
use sill_model::wallpaper::{
    CACHE_DIR, WallpaperCommand, WallpaperImage, WallpaperSource, Wallpapers,
};
use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch as channel};

use super::backdrop::ensure_backdrop;
use super::config::{config_path, resolve_source};
use super::{resize, watch};
use crate::env::Dirs;

/// `wallpaper` in sill's folder of `dirs`' cache directory.
pub(crate) fn cache_directory(dirs: &Dirs) -> PathBuf {
    dirs.sill_cache().join(CACHE_DIR)
}

/// Waits on `commands` while `listening`; parks forever once it is not (mirrors
/// `clock::run`'s own guard against a closed channel spinning `tokio::select!`).
async fn next_command(
    commands: &mut mpsc::UnboundedReceiver<WallpaperCommand>,
    listening: bool,
) -> Option<WallpaperCommand> {
    if listening {
        commands.recv().await
    } else {
        std::future::pending().await
    }
}

/// Waits on the debounced reload signal, or parks forever once there is none (no ancestor of
/// the config path existed at spawn time).
async fn next_reload(reload: &mut Option<mpsc::UnboundedReceiver<()>>) -> Option<()> {
    match reload {
        Some(receiver) => receiver.recv().await,
        None => std::future::pending().await,
    }
}

/// Where the service reads the source path and keeps its resized copies.
pub(crate) struct Places {
    pub(crate) config: PathBuf,
    pub(crate) cache: PathBuf,
}

/// Every command and every settled config change, until every [`Wallpapers`] receiver is gone.
pub(crate) async fn run(
    tx: channel::Sender<Wallpapers>,
    mut commands: mpsc::UnboundedReceiver<WallpaperCommand>,
    places: Places,
) {
    let rt = Handle::current();
    let reload = watch::watch(&config_path(&places.config), &rt);
    // Held for `run`'s whole life: dropping the watcher stops the watch.
    let (_watcher, mut reload) = match reload {
        Some((watcher, reload)) => (Some(watcher), Some(reload)),
        None => (None, None),
    };
    let mut source = resolve_source(&places.config);
    let mut sizes: HashMap<OutputName, (u32, u32)> = HashMap::new();
    let cache_dir = places.cache;
    let mut listening = true;
    while !tx.is_closed() {
        tokio::select! {
            command = next_command(&mut commands, listening) => match command {
                Some(WallpaperCommand::Prepare { output, width, height }) => {
                    sizes.insert(output.clone(), (width, height));
                    publish(&tx, &source, &output, width, height, &cache_dir, &rt).await;
                }
                Some(WallpaperCommand::Forget(output)) => {
                    sizes.remove(&output);
                    forget(&tx, &output);
                }
                Some(WallpaperCommand::Reload) => {
                    source = resolve_source(&places.config);
                    republish_all(&tx, &source, &sizes, &cache_dir, &rt).await;
                }
                None => listening = false,
            },
            signal = next_reload(&mut reload) => match signal {
                Some(()) => {
                    source = resolve_source(&places.config);
                    republish_all(&tx, &source, &sizes, &cache_dir, &rt).await;
                }
                None => reload = None,
            },
        }
    }
}

/// Re-prepare every tracked output against `source` (a `Reload`, from a command or a settled
/// watch signal).
async fn republish_all(
    tx: &channel::Sender<Wallpapers>,
    source: &WallpaperSource,
    sizes: &HashMap<OutputName, (u32, u32)>,
    cache_dir: &Path,
    rt: &Handle,
) {
    let outputs: Vec<(OutputName, u32, u32)> = sizes
        .iter()
        .map(|(output, &(width, height))| (output.clone(), width, height))
        .collect();
    for (output, width, height) in outputs {
        publish(tx, source, &output, width, height, cache_dir, rt).await;
    }
}

/// Prepare `output`'s images on the blocking pool and publish the result, replacing whatever
/// that output held before (dropping it when preparation fails or `source` is
/// [`WallpaperSource::None`], so a stale image is never shown as current).
async fn publish(
    tx: &channel::Sender<Wallpapers>,
    source: &WallpaperSource,
    output: &OutputName,
    width: u32,
    height: u32,
    cache_dir: &Path,
    rt: &Handle,
) {
    let source = source.clone();
    let cache_dir = cache_dir.to_path_buf();
    let prepared = rt
        .spawn_blocking(move || {
            let prepared = resize::prepare(&source, width, height, &cache_dir);
            // The lock screen's blurred copy, beside each image (a failure only costs the lock
            // screen its picture, never the wallpaper).
            for image in prepared.iter().flat_map(|(light, dark)| [light, dark]) {
                let _ = ensure_backdrop(image);
            }
            prepared
        })
        .await
        .unwrap_or_default();
    let output = output.clone();
    tx.send_modify(|wallpapers| {
        wallpapers.outputs.retain(|image| image.output != output);
        if let Some((light, dark)) = prepared {
            wallpapers.outputs.push(WallpaperImage {
                output,
                light,
                dark,
            });
        }
    });
}

/// Drop `output`'s entry, if it has one.
fn forget(tx: &channel::Sender<Wallpapers>, output: &OutputName) {
    tx.send_modify(|wallpapers| wallpapers.outputs.retain(|image| image.output != *output));
}

#[cfg(test)]
mod tests {
    use crate::service::spawn;
    use crate::wallpaper::WallpaperService;
    use sill_model::ids::OutputName;
    use sill_model::wallpaper::{WallpaperCommand, Wallpapers};

    /// Starting never panics (no compositor or config directory needed to start), and a
    /// `Prepare` in a scratch home with no wallpaper file settles to no entry at
    /// all — never a panic, a hang, or a half-written entry.
    #[tokio::test]
    async fn spawn_does_not_panic_and_prepare_settles_to_a_defined_state() {
        let rt = tokio::runtime::Handle::current();
        let scratch = sill_testkit::TempDir::new("wallpaper-spawn");
        let service =
            spawn::<WallpaperService>(&rt, &crate::env::Env::isolated(scratch.path()), ());
        let (mut wallpapers, control) = (service.state, service.control);
        assert_eq!(*wallpapers.borrow(), Wallpapers::default());
        control
            .send(WallpaperCommand::Prepare {
                output: OutputName("DP-1".to_owned()),
                width: 64,
                height: 64,
            })
            .unwrap_or_else(|e| panic!("{e}"));
        wallpapers.changed().await.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            wallpapers.borrow().outputs,
            Vec::new(),
            "no source: nothing prepared"
        );
    }
}
