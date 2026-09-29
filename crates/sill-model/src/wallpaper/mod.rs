//! The wallpaper: where it comes from and every output's cached images, as the wallpaper and lock
//! surfaces draw them.

mod cache;
mod command;
mod geometry;
mod model;

pub use cache::{CACHE_DIR, backdrop_file, cache_file_name, source_hash};
pub use command::WallpaperCommand;
pub use geometry::{CoverFit, cover_fit};
pub use model::{WallpaperImage, WallpaperSource, Wallpapers};
