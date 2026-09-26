//! The lock screen's backdrop (design/20 §1.9: "the Window material with a pre-blurred wallpaper
//! crop"; M11, FINDINGS F544): beside every prepared wallpaper image the service writes a
//! blurred copy of the same size, so the lock surface only loads a file and never blurs on the
//! UI thread (Blitz has no CSS `filter: blur`).

use std::path::{Path, PathBuf};

use crate::error::ServiceError;

/// The blur's standard deviation in buffer pixels at a 1920 px wide image, scaled with the
/// width: macOS's lock screen reads as a heavy, even blur with no shapes left.
pub const BACKDROP_SIGMA_AT_1920: f32 = 40.0;

/// Where the blurred copy of the cached `image` lives: the same directory, `-backdrop` before
/// the extension (`<hash>-<w>x<h>-backdrop.png`), so it follows the image's own cache key.
pub fn backdrop_file(image: &Path) -> PathBuf {
    todo!("M11 fill: a-lock ({})", image.display())
}

/// Write [`backdrop_file`] for `image` unless it already exists (blocking; the service runs it
/// on its blocking pool right after `ensure_cached`).
pub fn ensure_backdrop(image: &Path) -> Result<PathBuf, ServiceError> {
    todo!("M11 fill: a-lock ({})", image.display())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::backdrop_file;

    #[test]
    #[ignore = "M11 fill: a-lock"]
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
}
