//! Preparing one cached PNG: cover-fit the source into the target size with Lanczos3, and write
//! it atomically (temp file, then rename) so a document reading `file://<cache path>` never
//! sees a half-written image.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use image::imageops::FilterType;

use super::WallpaperSource;
use super::cache::{cache_file_name, source_hash};
use super::geometry::cover_fit;
use crate::error::ServiceError;

/// `path`'s cache entry at `width` x `height` in `cache_dir`, resizing it there first if no
/// entry by that name exists yet ("renders only on change": an unchanged source and size never
/// re-encodes). `Err` when the source cannot be read or decoded (a format `image` was not built
/// with — COSMIC's own default backgrounds are `.jxl`, `config.rs`'s module doc), or the cache
/// directory cannot be written.
pub fn ensure_cached(
    path: &Path,
    width: u32,
    height: u32,
    cache_dir: &Path,
) -> Result<PathBuf, ServiceError> {
    let metadata = std::fs::metadata(path).map_err(|source| ServiceError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let modified_unix_nanos = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|since| since.as_nanos())
        .unwrap_or_default();
    let hash = source_hash(path, metadata.len(), modified_unix_nanos);
    let cached = cache_dir.join(cache_file_name(hash, width, height));
    if cached.is_file() {
        return Ok(cached);
    }
    std::fs::create_dir_all(cache_dir).map_err(|source| ServiceError::Io {
        path: cache_dir.to_path_buf(),
        source,
    })?;
    resize_into(path, width, height, &cached)?;
    Ok(cached)
}

/// Cover-fit `source` into `width` x `height` and write it to `dest` (a sibling temp file, then
/// a rename, so a reader never opens a partial PNG).
fn resize_into(source: &Path, width: u32, height: u32, dest: &Path) -> Result<(), ServiceError> {
    let image = image::open(source).map_err(|error| ServiceError::Io {
        path: source.to_path_buf(),
        source: std::io::Error::other(error),
    })?;
    let fit = cover_fit((image.width(), image.height()), (width, height));
    let cropped = image
        .resize_exact(fit.scaled.0, fit.scaled.1, FilterType::Lanczos3)
        .crop_imm(fit.crop_x, fit.crop_y, width, height);
    let mut temp = dest.to_path_buf();
    temp.set_extension("png.tmp");
    cropped
        .save_with_format(&temp, image::ImageFormat::Png)
        .map_err(|error| ServiceError::Io {
            path: temp.clone(),
            source: std::io::Error::other(error),
        })?;
    std::fs::rename(&temp, dest).map_err(|source| ServiceError::Io {
        path: dest.to_path_buf(),
        source,
    })
}

/// The light and dark source paths `source` names, resized into `cache_dir` at `width` x
/// `height` and cached there. `None` for [`WallpaperSource::None`], or when either resize
/// fails (a missing file, or a format this crate's `image` build cannot decode — `config.rs`'s
/// module doc): a partial pair is never published, since [`super::WallpaperImage`] requires
/// both.
pub fn prepare(
    source: &WallpaperSource,
    width: u32,
    height: u32,
    cache_dir: &Path,
) -> Option<(PathBuf, PathBuf)> {
    let (light_src, dark_src) = match source {
        WallpaperSource::File(path) => (path.clone(), path.clone()),
        WallpaperSource::Pair { light, dark } => (light.clone(), dark.clone()),
        WallpaperSource::None => return None,
    };
    let light = ensure_cached(&light_src, width, height, cache_dir).ok()?;
    let dark = if dark_src == light_src {
        light.clone()
    } else {
        ensure_cached(&dark_src, width, height, cache_dir).ok()?
    };
    Some((light, dark))
}

#[cfg(test)]
mod tests {
    use image::{ImageBuffer, Rgba};

    use super::{WallpaperSource, ensure_cached, prepare};

    /// A 100 x 100 source: a 40 x 40 opaque-red block centred in an opaque-blue field, large
    /// enough (and far enough from every edge) that Lanczos3's ringing near a resize never
    /// reaches its own centre pixel, the way `dev/accept-wallpaper.sh` checks a real output's
    /// captured centre pixel against the source's.
    fn marked_source() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        ImageBuffer::from_fn(100, 100, |x, y| {
            if (30..70).contains(&x) && (30..70).contains(&y) {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 255])
            }
        })
    }

    #[test]
    fn a_resize_round_trip_keeps_the_source_centre_at_the_targets_centre() {
        let dir =
            std::env::temp_dir().join(format!("sill-wallpaper-resize-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        let source = dir.join("source.png");
        marked_source()
            .save(&source)
            .unwrap_or_else(|e| panic!("{e}"));
        let cache_dir = dir.join("cache");

        let cached = ensure_cached(&source, 250, 250, &cache_dir).unwrap_or_else(|e| panic!("{e}"));
        assert!(cached.is_file());
        let resized = image::open(&cached).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((resized.width(), resized.height()), (250, 250));
        let centre = resized.to_rgba8().get_pixel(125, 125).0;
        assert_eq!(
            centre,
            [255, 0, 0, 255],
            "the marked centre pixel travels through the resize"
        );

        // Re-running against the same source and size reuses the cache entry (same name,
        // same bytes) rather than writing again.
        let modified_before = std::fs::metadata(&cached)
            .unwrap_or_else(|e| panic!("{e}"))
            .modified();
        let again = ensure_cached(&source, 250, 250, &cache_dir).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(again, cached);
        let modified_after = std::fs::metadata(&cached)
            .unwrap_or_else(|e| panic!("{e}"))
            .modified();
        assert_eq!(modified_before.ok(), modified_after.ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_source_is_an_error_not_a_panic() {
        let dir =
            std::env::temp_dir().join(format!("sill-wallpaper-missing-{}", std::process::id()));
        let cache_dir = dir.join("cache");
        assert!(ensure_cached(&dir.join("nope.png"), 10, 10, &cache_dir).is_err());
    }

    #[test]
    fn a_file_source_reuses_one_cache_entry_for_both_schemes() {
        let dir = std::env::temp_dir().join(format!("sill-wallpaper-file-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        let source = dir.join("source.png");
        marked_source()
            .save(&source)
            .unwrap_or_else(|e| panic!("{e}"));
        let cache_dir = dir.join("cache");

        let (light, dark) = prepare(&WallpaperSource::File(source), 40, 20, &cache_dir)
            .unwrap_or_else(|| panic!("a decodable source prepares"));
        assert_eq!(light, dark, "one source, one cache entry for both schemes");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_source_prepares_nothing() {
        let cache_dir = std::env::temp_dir().join("sill-wallpaper-none-unused");
        assert_eq!(prepare(&WallpaperSource::None, 10, 10, &cache_dir), None);
    }
}
