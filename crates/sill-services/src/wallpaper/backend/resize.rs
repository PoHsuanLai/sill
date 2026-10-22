//! Preparing one cached PNG: cover-fit the source into the target size with Lanczos3, and write
//! it atomically (temp file, then rename) so a document reading `file://<cache path>` never
//! sees a half-written image.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use image::imageops::FilterType;
use image::{ImageReader, Limits};
use sill_model::wallpaper::{WallpaperSource, cache_file_name, cover_window, source_hash};

use crate::error::ServiceError;

/// The most pixels an output may have (an 8K wall of two monitors is about 66 million): the
/// cached copy is `width * height` pixels of memory while it is cut, so a bogus size is refused.
const MAX_TARGET_PIXELS: u64 = 100_000_000;

/// The widest and tallest source decoded. A decoder checks a header's size against these before it
/// allocates anything, so a hostile or accidental giant is refused cheaply.
const MAX_SOURCE_SIDE: u32 = 32_768;

/// The most memory a decoder may hold at once for one source (a 100 megapixel photo is 400 MiB).
const MAX_DECODE_BYTES: u64 = 1024 * 1024 * 1024;

/// `path`'s cache entry at `width` x `height` in `cache_dir`, resizing it there first if no
/// entry by that name exists yet ("renders only on change": an unchanged source and size never
/// re-encodes). `Err` when the source cannot be read or decoded (a format `image` was not built
/// with, such as `.jxl`), or the cache
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
    check_target(width, height, path)?;
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

/// Refuse an output size that is empty or beyond [`MAX_TARGET_PIXELS`].
fn check_target(width: u32, height: u32, path: &Path) -> Result<(), ServiceError> {
    let pixels = u64::from(width) * u64::from(height);
    if pixels == 0 || pixels > MAX_TARGET_PIXELS {
        return Err(ServiceError::Io {
            path: path.to_path_buf(),
            source: std::io::Error::other(format!("an output of {width} x {height} pixels")),
        });
    }
    Ok(())
}

/// `source` decoded within [`MAX_SOURCE_SIDE`] and [`MAX_DECODE_BYTES`].
fn decode_within_limits(source: &Path) -> Result<image::DynamicImage, ServiceError> {
    let failed = |error: &dyn std::fmt::Display| ServiceError::Io {
        path: source.to_path_buf(),
        source: std::io::Error::other(error.to_string()),
    };
    let mut reader = ImageReader::open(source).map_err(|error| ServiceError::Io {
        path: source.to_path_buf(),
        source: error,
    })?;
    let mut limits = Limits::no_limits();
    limits.max_image_width = Some(MAX_SOURCE_SIDE);
    limits.max_image_height = Some(MAX_SOURCE_SIDE);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    reader
        .with_guessed_format()
        .map_err(|error| failed(&error))?
        .decode()
        .map_err(|error| failed(&error))
}

/// Cover-fit `source` into `width` x `height` and write it to `dest` (a sibling temp file, then
/// a rename, so a reader never opens a partial PNG). Only the window of the source that fills
/// the target is cut out and resized, so a source of any aspect ratio costs at most the source
/// and the target in memory.
fn resize_into(source: &Path, width: u32, height: u32, dest: &Path) -> Result<(), ServiceError> {
    let image = decode_within_limits(source)?;
    let window = cover_window((image.width(), image.height()), (width, height));
    let cropped = image
        .crop_imm(window.x, window.y, window.width, window.height)
        .resize_exact(width, height, FilterType::Lanczos3);
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
/// module doc): a partial pair is never published, since [`sill_model::wallpaper::WallpaperImage`] requires
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

    use super::{ensure_cached, prepare};
    use sill_model::wallpaper::WallpaperSource;

    /// A 100 x 100 source: a 40 x 40 opaque-red block centred in an opaque-blue field, large
    /// enough (and far enough from every edge) that Lanczos3's ringing near a resize never
    /// reaches its own centre pixel, the way `dev/accept/wallpaper.sh` checks a real output's
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
        // Every format a wallpaper may be: png and webp are written here (the jpeg encoder is
        // lossy, so its centre pixel would only be near red).
        for name in ["source.png", "source.webp"] {
            round_trip(name);
        }
    }

    fn round_trip(name: &str) {
        let dir = std::env::temp_dir().join(format!(
            "sill-wallpaper-resize-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        let source = dir.join(name);
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
    fn a_hair_thin_panorama_fills_the_target_without_a_huge_allocation() {
        let dir = std::env::temp_dir().join(format!("sill-wallpaper-thin-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        let source = dir.join("thin.png");
        // 5000 x 1: scaled to cover a 400 x 225 output the old way it was about a gigabyte.
        ImageBuffer::from_fn(5000, 1, |x, _| {
            Rgba([u8::try_from(x % 256).unwrap_or_default(), 0, 0, 255])
        })
        .save(&source)
        .unwrap_or_else(|e| panic!("{e}"));
        let cached =
            ensure_cached(&source, 400, 225, &dir.join("cache")).unwrap_or_else(|e| panic!("{e}"));
        let filled = image::open(&cached).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((filled.width(), filled.height()), (400, 225));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_output_beyond_the_pixel_cap_is_refused() {
        let dir = std::env::temp_dir().join(format!("sill-wallpaper-huge-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        let source = dir.join("source.png");
        marked_source()
            .save(&source)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(ensure_cached(&source, 20_000, 20_000, &dir.join("cache")).is_err());
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
