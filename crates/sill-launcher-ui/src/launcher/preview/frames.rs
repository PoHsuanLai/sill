//! A video's picture in the pane, from the freedesktop thumbnail cache.
//!
//! The viewer's peek decodes no video (`anyview_peek::VideoFrames` is the seam it leaves). The
//! file manager or a thumbnailer usually made a small picture of the file already, under
//! `<cache>/thumbnails/<size>/<md5 of the file URI>.png`; this reads it through the viewer's
//! platform crate, the same cache and the same lookup as the viewer's own first frame. The cache
//! root is the daemon's injected `Dirs::cache`, never read from the process here. A missing
//! thumbnail, or one made for another version of the file, is no picture, and the pane shows
//! the facts card.

use std::path::Path;

use anyview_core::Source;
use anyview_image::Rgba8;
use anyview_peek::VideoFrames;
use anyview_platform::linux::FreedesktopThumbnails;
use anyview_platform::{Env, ThumbSize, ThumbnailCache};

/// The sizes tried, the larger first: the pane shows it at up to twice its own size.
const SIZES: [ThumbSize; 2] = [ThumbSize::Large, ThumbSize::Normal];

/// The thumbnail cache as the peek's source of a video's frame.
#[derive(Debug, Clone)]
pub struct ThumbnailFrames {
    cache: FreedesktopThumbnails,
}

impl ThumbnailFrames {
    /// The cache under `cache_dir` (the person's `XDG_CACHE_HOME`): `<cache_dir>/thumbnails`.
    pub fn in_cache(cache_dir: &Path) -> ThumbnailFrames {
        // The platform crate reads only the cache directory of its `Env`; the rest is isolated
        // and refuses to reach a bus or start a program.
        let mut env = Env::isolated(cache_dir);
        env.dirs.cache = cache_dir.to_path_buf();
        ThumbnailFrames {
            cache: FreedesktopThumbnails::new(&env),
        }
    }
}

impl VideoFrames for ThumbnailFrames {
    fn frame(&self, source: &Source) -> Option<Rgba8> {
        let stamp = source.stamp();
        SIZES.iter().find_map(|size| {
            let found = self
                .cache
                .lookup(source.path(), &stamp, *size)
                .ok()
                .flatten()?;
            Rgba8::new(found.size(), found.rgba().to_vec()).ok()
        })
    }
}
