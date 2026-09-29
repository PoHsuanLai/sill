//! The thumbnail's picture: the screenshot downscaled to at most [`PREVIEW_MAX`] px on its long
//! side, a PNG in `$XDG_CACHE_HOME/sill/shots/` named by [`preview_name`], written to a temp
//! file and renamed so a document never reads half of it. Blocking: run on the pool.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use sill_model::screenshots::preview_name;

use crate::env::Dirs;
use crate::error::ServiceError;

/// The preview's long side in px: twice the thumbnail's width, for a 2x output.
pub const PREVIEW_MAX: u32 = 480;

/// Where previews go: `shots` in sill's folder of `dirs`' cache directory.
pub(crate) fn cache_in(dirs: &Dirs) -> PathBuf {
    dirs.sill_cache().join("shots")
}

/// `from`'s preview in `cache`.
pub(crate) fn make_in(from: &Path, cache: &Path) -> Result<PathBuf, ServiceError> {
    let metadata = std::fs::metadata(from).map_err(|e| io(from, e))?;
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
        .and_then(|since| i64::try_from(since.as_millis()).ok())
        .unwrap_or_default();
    let dest = cache.join(preview_name(from, metadata.len(), modified_ms));
    if dest.is_file() {
        return Ok(dest);
    }
    std::fs::create_dir_all(cache).map_err(|e| io(cache, e))?;
    let image = image::open(from).map_err(|e| io(from, std::io::Error::other(e)))?;
    let small = match image.width().max(image.height()) > PREVIEW_MAX {
        true => image.thumbnail(PREVIEW_MAX, PREVIEW_MAX),
        false => image,
    };
    let temp = dest.with_extension("png.tmp");
    small
        .save_with_format(&temp, image::ImageFormat::Png)
        .map_err(|e| io(&temp, std::io::Error::other(e)))?;
    std::fs::rename(&temp, &dest).map_err(|e| io(&dest, e))?;
    Ok(dest)
}

fn io(path: &Path, source: std::io::Error) -> ServiceError {
    ServiceError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::{PREVIEW_MAX, make_in};

    #[test]
    fn a_large_screenshot_is_downscaled_once_and_a_broken_one_fails() {
        let dir = std::env::temp_dir().join(format!("sill-shot-preview-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp dir"); // test-only
        let shot = dir.join("Screenshot_1.png");
        image::RgbImage::new(1920, 1080).save(&shot).expect("a png"); // test-only
        let cache = dir.join("cache");
        let made = make_in(&shot, &cache).expect("a preview"); // test-only
        let preview = image::open(&made).expect("it reads"); // test-only
        assert_eq!((preview.width(), preview.height()), (PREVIEW_MAX, 270));
        assert_eq!(make_in(&shot, &cache).ok(), Some(made), "cached");
        let broken = dir.join("Screenshot_2.png");
        std::fs::write(&broken, b"not a png").expect("written"); // test-only
        assert!(make_in(&broken, &cache).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
