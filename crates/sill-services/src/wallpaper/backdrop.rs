//! The lock screen's backdrop (design/20 §1.9: "the Window material with a pre-blurred wallpaper
//! crop"): beside every prepared wallpaper image the service writes a blurred copy of the same
//! size, so the lock surface only loads a file and never blurs on the
//! UI thread (Blitz has no CSS `filter: blur`).

use std::path::{Path, PathBuf};

use image::RgbaImage;
use image::imageops::{self, FilterType};

use crate::error::ServiceError;

/// The blur's standard deviation in buffer pixels at a 1920 px wide image, scaled with the
/// width: macOS's lock screen reads as a heavy, even blur with no shapes left.
pub const BACKDROP_SIGMA_AT_1920: f32 = 40.0;

/// Where the blurred copy of the cached `image` lives: the same directory, `-backdrop` before
/// the extension (`<hash>-<w>x<h>-backdrop.png`), so it follows the image's own cache key.
pub fn backdrop_file(image: &Path) -> PathBuf {
    let stem = image
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = match image.extension() {
        Some(extension) => format!("{stem}-backdrop.{}", extension.to_string_lossy()),
        None => format!("{stem}-backdrop"),
    };
    image.with_file_name(name)
}

/// Write [`backdrop_file`] for `image` unless it already exists (blocking; the service runs it
/// on its blocking pool right after `ensure_cached`).
pub fn ensure_backdrop(image: &Path) -> Result<PathBuf, ServiceError> {
    let backdrop = backdrop_file(image);
    if backdrop.is_file() {
        return Ok(backdrop);
    }
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |error: image::ImageError| ServiceError::Io {
            path,
            source: std::io::Error::other(error),
        }
    };
    let source = image::open(image).map_err(io(image))?.to_rgba8();
    let blurred = blur(&source);
    let mut temp = backdrop.clone();
    temp.set_extension("png.tmp");
    blurred
        .save_with_format(&temp, image::ImageFormat::Png)
        .map_err(io(&temp))?;
    std::fs::rename(&temp, &backdrop).map_err(|source| ServiceError::Io {
        path: backdrop.clone(),
        source,
    })?;
    Ok(backdrop)
}

/// A heavy, even blur at [`sigma_for`]: downscale, blur there, scale back (the look is a blur
/// with no shapes left, so the detail lost to the downscale is never seen, and a 40 px sigma at
/// full size would take seconds).
fn blur(source: &RgbaImage) -> RgbaImage {
    let (width, height) = source.dimensions();
    let small = (width / SHRINK).max(1);
    let small_height = (height / SHRINK).max(1);
    let reduced = imageops::resize(source, small, small_height, FilterType::Triangle);
    let sigma = sigma_for(width) / SHRINK_F;
    let blurred = imageops::fast_blur(&reduced, sigma);
    imageops::resize(&blurred, width, height, FilterType::Triangle)
}

/// How much the blur shrinks the image first.
const SHRINK: u32 = 8;
const SHRINK_F: f32 = 8.0;

/// The blur's sigma for an image `width` pixels wide: [`BACKDROP_SIGMA_AT_1920`] scaled.
fn sigma_for(width: u32) -> f32 {
    let width = u16::try_from(width).map_or(f32::from(u16::MAX), f32::from);
    BACKDROP_SIGMA_AT_1920 * width / 1920.0
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{backdrop_file, ensure_backdrop};

    #[test]
    fn the_backdrop_sits_beside_its_image() {
        let cases = [
            (
                "/home/ada/.cache/sill/wallpaper/00000000deadbeef-1920x1080.png",
                "/home/ada/.cache/sill/wallpaper/00000000deadbeef-1920x1080-backdrop.png",
            ),
            ("/tmp/a.png", "/tmp/a-backdrop.png"),
        ];
        for (image, want) in cases {
            assert_eq!(backdrop_file(Path::new(image)), PathBuf::from(want));
        }
    }

    #[test]
    fn a_backdrop_is_written_once_at_the_images_size_and_blurred() {
        let dir = std::env::temp_dir().join(format!("sill-backdrop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        let image = dir.join("aa-160x90.png");
        image::RgbaImage::from_fn(160, 90, |x, _| {
            if x < 80 {
                image::Rgba([0, 0, 0, 255])
            } else {
                image::Rgba([255, 255, 255, 255])
            }
        })
        .save(&image)
        .unwrap_or_else(|e| panic!("{e}"));
        let written = ensure_backdrop(&image).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(written, dir.join("aa-160x90-backdrop.png"));
        let blurred = image::open(&written)
            .unwrap_or_else(|e| panic!("{e}"))
            .to_rgba8();
        assert_eq!(blurred.dimensions(), (160, 90));
        let edge = blurred.get_pixel(80, 45).0[0];
        assert!((40..=215).contains(&edge), "the hard edge is soft: {edge}");
        let modified = || std::fs::metadata(&written).and_then(|m| m.modified()).ok();
        let before = modified();
        let again = ensure_backdrop(&image).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((again, modified()), (written, before));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
