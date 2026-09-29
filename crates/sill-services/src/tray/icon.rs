//! Turning a [`TrayIcon`] into the art the bar draws: a pixmap's PNG bytes, or the file a named
//! icon resolves to in the installed theme, with what design/08-ICONS.md §1.5 can tell from
//! the name alone (step 1: a freedesktop `*-symbolic` icon is symbolic). The chroma test for the
//! rest (step 2) is quire's `ds::icon::classify`, which the bar runs on the PNG bytes: this
//! crate does not reach `ds` (`scripts/check-boundary.sh`). The URL itself is quire's too
//! (`ds::IconUrl::png` does the base64, `IconUrl::file` the path).

use std::path::{Path, PathBuf};

use image::{ExtendedColorType, ImageEncoder};
use sill_model::tray::{Pixmap, TrayIcon};

/// What a status item's icon is, as far as this side can tell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconArt {
    /// PNG bytes (an SNI pixmap, or a theme icon that is a PNG file): symbolic or not is the
    /// chroma test's call.
    Png(Vec<u8>),
    /// A theme file whose name is freedesktop symbolic: drawn in the text colour.
    SymbolicFile(PathBuf),
    /// Any other theme file (an SVG): drawn as it is.
    ImageFile(PathBuf),
}

/// `icon`'s art, a named icon resolved at `size_px` against the installed theme. `None` for
/// [`TrayIcon::None`], a pixmap that fails to encode, a name the theme does not have, or a PNG
/// file that cannot be read.
pub fn icon_art(icon: &TrayIcon, size_px: u16) -> Option<IconArt> {
    match icon {
        TrayIcon::None => None,
        TrayIcon::Named(name) => {
            let path = freedesktop_icons::lookup(name).with_size(size_px).find()?;
            named_art(name, path)
        }
        TrayIcon::Pixmap(pixmap) => pixmap_png(pixmap).map(IconArt::Png),
    }
}

/// The art for the theme file `path` that `name` resolved to: symbolic by either name, else a
/// PNG's bytes, else the file as an image.
fn named_art(name: &str, path: PathBuf) -> Option<IconArt> {
    if symbolic_name(name)
        || path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(symbolic_name)
    {
        return Some(IconArt::SymbolicFile(path));
    }
    if is_png(&path) {
        return std::fs::read(&path).ok().map(IconArt::Png);
    }
    Some(IconArt::ImageFile(path))
}

/// design/08 §1.5 step 1: the freedesktop suffix, compared whole after the last `-`.
fn symbolic_name(name: &str) -> bool {
    name.rsplit_once('-')
        .is_some_and(|(stem, suffix)| !stem.is_empty() && suffix == "symbolic")
}

fn is_png(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("png"))
}

/// An SNI `IconPixmap`'s raw ARGB32 bytes (network byte order: A, R, G, B per pixel), PNG
/// encoded. `None` when the buffer's length does not match `width * height * 4`, or the
/// encoder refuses it.
fn pixmap_png(pixmap: &Pixmap) -> Option<Vec<u8>> {
    let mut rgba = Vec::with_capacity(pixmap.argb32.len());
    let (words, _remainder) = pixmap.argb32.as_chunks::<4>();
    for &[a, r, g, b] in words {
        rgba.extend_from_slice(&[r, g, b, a]);
    }
    if rgba.len() != (pixmap.width as usize) * (pixmap.height as usize) * 4 {
        return None;
    }
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&rgba, pixmap.width, pixmap.height, ExtendedColorType::Rgba8)
        .ok()?;
    Some(png)
}

#[cfg(test)]
mod tests {
    use super::{IconArt, icon_art, named_art, pixmap_png, symbolic_name};
    use sill_model::tray::{Pixmap, TrayIcon};
    use std::path::PathBuf;

    #[test]
    fn a_two_by_one_pixmap_becomes_a_png() {
        // Two pixels, ARGB32 network order: opaque red, then opaque blue.
        let pixmap = Pixmap {
            width: 2,
            height: 1,
            argb32: vec![0xFF, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF],
        };
        let png = pixmap_png(&pixmap).expect("a well-formed buffer encodes");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let decoded = image::load_from_memory(&png).expect("decodes").to_rgba8();
        assert_eq!(decoded.get_pixel(0, 0).0, [0xFF, 0x00, 0x00, 0xFF]);
        assert_eq!(decoded.get_pixel(1, 0).0, [0x00, 0x00, 0xFF, 0xFF]);
    }

    #[test]
    fn a_short_buffer_is_refused() {
        let pixmap = Pixmap {
            width: 2,
            height: 2,
            argb32: vec![0xFF, 0x00, 0x00, 0x00], // one pixel's worth, not four
        };
        assert_eq!(pixmap_png(&pixmap), None);
        assert_eq!(icon_art(&TrayIcon::Pixmap(pixmap), 22), None);
    }

    #[test]
    fn none_has_no_art() {
        assert_eq!(icon_art(&TrayIcon::None, 22), None);
    }

    #[test]
    fn the_symbolic_suffix_is_matched_whole() {
        const CASES: &[(&str, bool)] = &[
            ("input-keyboard-symbolic", true),
            ("network-wireless-symbolic", true),
            ("symbolic", false),
            ("-symbolic", false),
            ("input-keyboard", false),
            ("toshy-symbolical", false),
            ("app-symbolic-light", false),
        ];
        for &(name, want) in CASES {
            assert_eq!(symbolic_name(name), want, "{name}");
        }
    }

    #[test]
    fn a_named_icon_is_symbolic_by_either_name_and_an_image_otherwise() {
        let file = |p: &str| PathBuf::from(p);
        let cases = [
            (
                "update-high-symbolic",
                file("/i/update-high.svg"),
                IconArt::SymbolicFile(file("/i/update-high.svg")),
            ),
            (
                "update-high",
                file("/i/update-high-symbolic.svg"),
                IconArt::SymbolicFile(file("/i/update-high-symbolic.svg")),
            ),
            (
                "update-high",
                file("/i/update-high.svg"),
                IconArt::ImageFile(file("/i/update-high.svg")),
            ),
        ];
        for (name, path, want) in cases {
            assert_eq!(named_art(name, path), Some(want), "{name}");
        }
        assert_eq!(
            named_art("gone", file("/nonexistent/gone.png")),
            None,
            "an unreadable PNG has no art"
        );
    }
}
