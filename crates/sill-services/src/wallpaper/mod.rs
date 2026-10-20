//! The wallpaper: one pre-resized PNG per output size and scheme, cached in
//! `$XDG_CACHE_HOME/sill/wallpaper/<hash>-<w>x<h>.png` (image 0.25 Lanczos3; light/dark cross-fade
//! over t-big; renders only on change).
//!
//! The picture is the `wallpaper.picture` setting, which the daemon sends as
//! `WallpaperCommand::Picture`; while it is empty the old `sill/wallpaper-source` file in the
//! config directory is read instead (a read-only migration).
//!
//! `spawn`'s task owns everything effectful: `backend::config::resolve_source` resolves the source
//! path (I/O, `config`), `backend::watch::watch` notices the old file change (I/O, `watch`), and
//! `backend::resize::prepare` resizes it onto the blocking pool (I/O, `resize`) using the pure
//! geometry in `geometry` and the pure naming in `cache`.

mod backend;

use sill_model::wallpaper::{WallpaperCommand, Wallpapers};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The wallpapers as a [`Service`]: resized on the blocking pool, published as cached paths.
/// The old source file is read under `env`'s config directory, the resized copies kept in its cache.
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
        let places = backend::Places {
            config: env.dirs.config.clone(),
            cache: backend::cache_directory(&env.dirs),
        };
        async move {
            backend::run(io.state, io.commands, places).await;
            Ok(())
        }
    }
}
