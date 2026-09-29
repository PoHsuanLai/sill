//! The wallpaper: one pre-resized PNG per output size and scheme, cached in
//! `$XDG_CACHE_HOME/sill/wallpaper/<hash>-<w>x<h>.png` (image 0.25 Lanczos3; light/dark cross-fade
//! over t-big; renders only on change).
//!
//! design/22 has no key naming the wallpaper file; sill reads COSMIC's own background config
//! (`com.system76.CosmicBackground`) until one is added.
//!
//! [`spawn`]'s task owns everything effectful: [`config::resolve_source`] reads the source path
//! (I/O, [`config`]), [`watch::watch`] notices it change (I/O, [`watch`]), and
//! [`resize::prepare`] resizes it onto the blocking pool (I/O, [`resize`]) using the pure
//! geometry in [`geometry`] and the pure naming in [`cache`].

mod backdrop;
mod cache;
mod config;
mod geometry;
mod resize;
mod watch;

pub use backdrop::{BACKDROP_SIGMA_AT_1920, backdrop_file, ensure_backdrop};
pub use cache::{CACHE_DIR, cache_file_name, source_hash};
pub use config::{config_path, resolve_source};
pub use geometry::{CoverFit, cover_fit};

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch as channel};

use crate::cosmic_wl::OutputName;
use crate::env::{Dirs, Env};
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// Where a wallpaper comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum WallpaperSource {
    /// One image for both schemes.
    File(PathBuf),
    /// A light and a dark image, cross-faded when the scheme changes.
    Pair { light: PathBuf, dark: PathBuf },
    /// No image configured: the surface paints the `--paper` token.
    None,
}

/// Every output's prepared images.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Wallpapers {
    pub outputs: Vec<WallpaperImage>,
}

/// One output's cached images, at its buffer size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WallpaperImage {
    pub output: OutputName,
    pub light: PathBuf,
    pub dark: PathBuf,
}

/// What the wallpaper service can be told.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum WallpaperCommand {
    /// An output appeared or changed size: prepare its images (buffer pixels).
    Prepare {
        output: OutputName,
        width: u32,
        height: u32,
    },
    /// An output went away.
    Forget(OutputName),
    /// Re-read the source.
    Reload,
}

/// `wallpaper` in sill's folder of `dirs`' cache directory.
fn cache_directory(dirs: &Dirs) -> PathBuf {
    dirs.sill_cache().join(CACHE_DIR)
}

/// The wallpapers as a [`Service`]: resized on the blocking pool, published as cached paths.
/// COSMIC's record is read under `env`'s config directory, the resized copies kept in its cache.
#[derive(Debug)]
pub enum WallpaperService {}

impl Service for WallpaperService {
    const NAME: &'static str = "wallpaper";
    type State = Wallpapers;
    type Command = WallpaperCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let places = Places {
            config: env.dirs.config.clone(),
            cache: cache_directory(&env.dirs),
        };
        async move {
            run(io.state, io.commands, places).await;
            Ok(())
        }
    }
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

/// Where the service reads COSMIC's record and keeps its resized copies.
struct Places {
    config: PathBuf,
    cache: PathBuf,
}

/// Every command and every settled config change, until every [`Wallpapers`] receiver is gone.
async fn run(
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
    use super::{WallpaperCommand, WallpaperService, Wallpapers};
    use crate::cosmic_wl::OutputName;
    use crate::service::spawn;

    /// Starting never panics (no compositor or config directory needed to start), and a
    /// `Prepare` in a scratch home with no COSMIC background config settles to no entry at
    /// all — never a panic, a hang, or a half-written entry.
    #[tokio::test]
    async fn spawn_does_not_panic_and_prepare_settles_to_a_defined_state() {
        let rt = tokio::runtime::Handle::current();
        let scratch = crate::env::scratch::Scratch::new("wallpaper-spawn");
        let service = spawn::<WallpaperService>(&rt, &scratch.env(), ());
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
