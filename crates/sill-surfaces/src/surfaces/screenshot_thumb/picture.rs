//! What quire's `ShotThumbnail` is given to draw: the shot's picture as an `ImageSource` (the
//! 480 px preview, or the file itself when no preview could be made) and its size in pixels,
//! read from the PNG's header, so the card takes the picture's shape. A picture that is not a
//! PNG (a JPEG from another tool) is drawn in the 16:10 box, letterboxed.

use std::io::Read;
use std::path::{Path, PathBuf};

use ds::{ImageSize, ImageSource};
use sill_services::{Shot, ShotPreview};

/// The shape a picture is drawn in when its header says nothing: the surface's 16:10.
const UNKNOWN: ImageSize = ImageSize {
    width: 16,
    height: 10,
};

/// A PNG's signature, then the IHDR chunk's length and type.
const PNG_HEAD: [u8; 16] = [
    0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
];

/// A shot's picture, ready for the card.
#[derive(Debug, Clone, PartialEq)]
pub struct ThumbPicture {
    /// The screenshot file itself (what a drag carries and a click opens).
    pub file: PathBuf,
    pub image: ImageSource,
    pub size: ImageSize,
}

/// The picture a card draws for `shot`, or `None` while its preview is pending or when its path
/// is not absolute.
pub fn picture_of(shot: &Shot) -> Option<ThumbPicture> {
    let path = match &shot.preview {
        ShotPreview::Ready(preview) => preview.as_path(),
        ShotPreview::Original => shot.path.as_path(),
        ShotPreview::Pending => return None,
    };
    Some(ThumbPicture {
        file: shot.path.clone(),
        image: ImageSource::file(path).ok()?,
        size: size_of_file(path),
    })
}

/// The pixel size in `path`'s PNG header, or the 16:10 stand-in.
fn size_of_file(path: &Path) -> ImageSize {
    let mut head = [0u8; 24];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut head))
        .ok()
        .and_then(|()| png_size(&head))
        .unwrap_or(UNKNOWN)
}

/// The width and height a PNG's first 24 bytes give (its IHDR), if they are a PNG's with no
/// zero side.
pub fn png_size(head: &[u8]) -> Option<ImageSize> {
    let word = |at: usize| {
        let bytes: [u8; 4] = head.get(at..at + 4)?.try_into().ok()?;
        Some(u32::from_be_bytes(bytes))
    };
    let size = ImageSize {
        width: word(16)?,
        height: word(20)?,
    };
    (head.get(..16) == Some(&PNG_HEAD[..]) && size.width > 0 && size.height > 0).then_some(size)
}

#[cfg(test)]
mod tests {
    use ds::ImageSize;

    use super::{PNG_HEAD, png_size};

    fn head(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = PNG_HEAD.to_vec();
        bytes.extend(width.to_be_bytes());
        bytes.extend(height.to_be_bytes());
        bytes
    }

    #[test]
    fn a_png_header_gives_its_size() {
        let mut jpeg = head(480, 300);
        jpeg[0] = 0xff;
        let cases = [
            (
                head(480, 300),
                Some(ImageSize {
                    width: 480,
                    height: 300,
                }),
            ),
            (
                head(2560, 1600),
                Some(ImageSize {
                    width: 2560,
                    height: 1600,
                }),
            ),
            (head(0, 300), None),
            (jpeg, None),
            (head(480, 300)[..20].to_vec(), None),
            (Vec::new(), None),
        ];
        for (bytes, want) in cases {
            assert_eq!(png_size(&bytes), want, "{bytes:?}");
        }
    }
}
