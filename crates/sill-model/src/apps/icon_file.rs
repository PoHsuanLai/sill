//! Which files an absolute `Icon=` may name. A desktop entry is another program's text, so the
//! file it names is read only when it is an image of a sane size.

use std::path::Path;

/// The largest raster or other icon file read: an app icon is a few kilobytes, a large one a few
/// hundred.
pub const ICON_FILE_MAX_BYTES: u64 = 8 * 1024 * 1024;

/// The largest SVG read: a drawing of an icon, not a document.
pub const SVG_FILE_MAX_BYTES: u64 = 1024 * 1024;

/// The extensions of the files an icon is drawn from, compared without regard to case.
const RASTER_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

/// Whether a file at `path` of `len` bytes may be an app's icon: it has an image extension and is
/// no larger than the cap for its kind. That it is a regular file is for the caller to have
/// checked (it takes a read of the file's metadata).
pub fn icon_file_fits(path: &Path, len: u64) -> bool {
    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    if extension.eq_ignore_ascii_case("svg") {
        return len <= SVG_FILE_MAX_BYTES;
    }
    RASTER_EXTENSIONS
        .iter()
        .any(|raster| extension.eq_ignore_ascii_case(raster))
        && len <= ICON_FILE_MAX_BYTES
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{ICON_FILE_MAX_BYTES, SVG_FILE_MAX_BYTES, icon_file_fits};

    #[test]
    fn only_an_image_within_its_kinds_cap_may_be_an_icon() {
        let cases: [(&str, &str, u64, bool); 11] = [
            ("a png", "/opt/app/icon.png", 4_000, true),
            ("a jpeg", "/opt/app/icon.JPG", 4_000, true),
            ("an svg", "/snap/x/current/meta/gui/icon.svg", 4_000, true),
            ("a raster at its cap", "/o/i.png", ICON_FILE_MAX_BYTES, true),
            (
                "a raster past its cap",
                "/o/i.png",
                ICON_FILE_MAX_BYTES + 1,
                false,
            ),
            ("an svg at its cap", "/o/i.svg", SVG_FILE_MAX_BYTES, true),
            (
                "an svg past its cap",
                "/o/i.svg",
                SVG_FILE_MAX_BYTES + 1,
                false,
            ),
            ("no extension", "/opt/app/icon", 10, false),
            ("a text file", "/opt/app/notes.txt", 10, false),
            (
                "a script in a folder named like an image",
                "/opt/app.png/run.sh",
                10,
                false,
            ),
            ("a dot file", "/opt/.png", 10, false),
        ];
        for (name, path, len, want) in cases {
            assert_eq!(icon_file_fits(Path::new(path), len), want, "{name}");
        }
    }
}
