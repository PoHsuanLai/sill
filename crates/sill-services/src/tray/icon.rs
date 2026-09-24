//! Turning a [`TrayIcon`] into something the bar can put in a stylesheet or a `data:`/`file:`
//! URL: a pixmap PNG-encoded and base64'd (shell-host's `LocalNet` decodes `data:` URIs), or a
//! named icon resolved against the installed theme (shell-host's `LocalNet` reads `file:`
//! URLs). Recolouring a symbolic icon to the ink token is a Blitz `mask-image`, which has no
//! home in `ds` yet (FINDINGS.md "M1 svc-tray", quire gap Q6): these functions only get the
//! bytes and the path, never a colour.

use image::{ExtendedColorType, ImageEncoder};

use super::model::{Pixmap, TrayIcon};

/// `icon` as a URL the bar's document can load: a pixmap becomes a `data:image/png;base64,...`
/// URL, a named icon becomes a `file://...` URL resolved at `size_px` against the installed
/// icon theme. `None` for [`TrayIcon::None`], a pixmap that fails to encode, or a name the
/// theme does not have.
pub fn icon_url(icon: &TrayIcon, size_px: u16) -> Option<String> {
    match icon {
        TrayIcon::None => None,
        TrayIcon::Named(name) => named_icon_file_url(name, size_px),
        TrayIcon::Pixmap(pixmap) => pixmap_png_data_url(pixmap),
    }
}

/// A freedesktop-compliant icon name, resolved against the installed theme at `size_px` and
/// turned into a `file://` URL shell-host's `LocalNet` can read directly. `None` when the
/// theme has no icon by that name.
fn named_icon_file_url(name: &str, size_px: u16) -> Option<String> {
    let path = freedesktop_icons::lookup(name).with_size(size_px).find()?;
    Some(format!("file://{}", path.display()))
}

/// An SNI `IconPixmap`'s raw ARGB32 bytes (network byte order: A, R, G, B per pixel), PNG
/// encoded and wrapped as a `data:` URL. `None` when the buffer's length does not match
/// `width * height * 4`, or the encoder refuses it.
fn pixmap_png_data_url(pixmap: &Pixmap) -> Option<String> {
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
    Some(format!("data:image/png;base64,{}", base64_encode(&png)))
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// A minimal RFC 4648 base64 encoder (standard alphabet, `=` padding). Not in the pinned
/// dependency block, so hand-rolled rather than adding a crate for a handful of small icons —
/// the same call shell-host's `LocalNet` made for the matching decoder
/// (`shell-host/src/dom/net.rs`).
fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let word = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((word >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((word >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[((word >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(word & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{base64_encode, icon_url, pixmap_png_data_url};
    use crate::tray::model::{Pixmap, TrayIcon};

    #[test]
    fn base64_matches_known_vectors() {
        let cases: &[(&[u8], &str)] = &[
            (b"", ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"hello", "aGVsbG8="),
        ];
        for (input, want) in cases {
            assert_eq!(base64_encode(input), *want, "{input:?}");
        }
    }

    #[test]
    fn a_two_by_one_pixmap_becomes_a_data_url() {
        // Two pixels, ARGB32 network order: opaque red, then opaque blue.
        let pixmap = Pixmap {
            width: 2,
            height: 1,
            argb32: vec![0xFF, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF],
        };
        let url = pixmap_png_data_url(&pixmap).expect("a well-formed buffer encodes");
        assert!(url.starts_with("data:image/png;base64,"), "{url}");
    }

    #[test]
    fn a_short_buffer_is_refused() {
        let pixmap = Pixmap {
            width: 2,
            height: 2,
            argb32: vec![0xFF, 0x00, 0x00, 0x00], // one pixel's worth, not four
        };
        assert_eq!(pixmap_png_data_url(&pixmap), None);
    }

    #[test]
    fn none_has_no_url() {
        assert_eq!(icon_url(&TrayIcon::None, 22), None);
    }
}
