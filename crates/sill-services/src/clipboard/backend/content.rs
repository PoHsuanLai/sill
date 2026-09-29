//! What a selection read whole becomes, and how much of it may be read. Pure: the live loop
//! reads the bytes, this decides whether they are kept. Nothing here prints what it is given.

use std::io::Cursor;
use std::sync::Arc;

use image::ImageReader;
use sill_launcher::Mime;
use sill_model::clipboard::{ClipContent, ClipImage, ClipText, ImageCap, MimePick, Skipped};

/// The most text read from one selection, in bytes; a longer copy is skipped as
/// [`Skipped::TooLarge`]. Proposed, with no settings key.
pub(crate) const TEXT_LIMIT: ReadLimit = ReadLimit(1 << 20);

/// The most bytes read before a selection is given up as too large.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReadLimit(pub(crate) u64);

/// How much of `pick` may be read under the policy's image cap.
pub(crate) fn limit_for(pick: &MimePick, images: ImageCap) -> ReadLimit {
    match pick {
        MimePick::Text(_) => TEXT_LIMIT,
        MimePick::Image(_) => ReadLimit(images.0),
    }
}

/// `bytes`, read whole as `pick`, as history content: text that is valid UTF-8 and not blank,
/// or an image whose size its header gives.
pub(crate) fn content_of(pick: &MimePick, bytes: Vec<u8>) -> Result<ClipContent, Skipped> {
    match pick {
        MimePick::Text(_) => text_of(bytes).map(ClipContent::Text),
        MimePick::Image(mime) => image_of(mime, bytes).map(ClipContent::Image),
    }
}

fn text_of(bytes: Vec<u8>) -> Result<ClipText, Skipped> {
    let text = String::from_utf8(bytes).map_err(|_| Skipped::Empty)?;
    if text.trim().is_empty() {
        return Err(Skipped::Empty);
    }
    Ok(ClipText(Arc::from(text)))
}

fn image_of(mime: &str, bytes: Vec<u8>) -> Result<ClipImage, Skipped> {
    if bytes.is_empty() {
        return Err(Skipped::Empty);
    }
    let (width, height) = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .ok_or(Skipped::NoKnownType)?;
    Ok(ClipImage {
        mime: Mime(mime.to_ascii_lowercase()),
        bytes: Arc::from(bytes),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{ImageFormat, RgbaImage};

    use super::{ReadLimit, TEXT_LIMIT, content_of, limit_for};
    use sill_model::clipboard::MimePick;
    use sill_model::clipboard::{ClipContent, ImageCap, Skipped};

    fn text(mime: &str) -> MimePick {
        MimePick::Text(mime.to_owned())
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        RgbaImage::new(width, height)
            .write_to(&mut out, ImageFormat::Png)
            .expect("an in-memory PNG encodes");
        out.into_inner()
    }

    /// What became of some bytes, without the content itself: a size, or the skip.
    type Shape = Result<(u32, u32), Skipped>;

    fn shape(pick: &MimePick, bytes: Vec<u8>) -> Shape {
        content_of(pick, bytes).map(|content| match content {
            ClipContent::Text(t) => (u32::try_from(t.0.len()).unwrap_or(u32::MAX), 0),
            ClipContent::Image(i) => (i.width, i.height),
        })
    }

    #[test]
    fn text_is_kept_when_it_is_utf8_and_not_blank() {
        let cases: Vec<(Vec<u8>, Shape)> = vec![
            (b"made-up words".to_vec(), Ok((13, 0))),
            ("caf\u{e9}".as_bytes().to_vec(), Ok((5, 0))),
            (Vec::new(), Err(Skipped::Empty)),
            (b" \n\t".to_vec(), Err(Skipped::Empty)),
            (vec![0xff, 0xfe, 0x41], Err(Skipped::Empty)),
        ];
        for (bytes, want) in cases {
            assert_eq!(shape(&text("text/plain"), bytes), want);
        }
    }

    #[test]
    fn an_image_is_kept_with_the_size_its_header_gives() {
        let pick = MimePick::Image("image/PNG".to_owned());
        assert_eq!(shape(&pick, png(3, 2)), Ok((3, 2)));
        assert_eq!(shape(&pick, Vec::new()), Err(Skipped::Empty));
        assert_eq!(
            shape(&pick, b"not an image".to_vec()),
            Err(Skipped::NoKnownType)
        );
        match content_of(&pick, png(1, 1)) {
            Ok(ClipContent::Image(image)) => assert_eq!(image.mime.0, "image/png"),
            other => panic!("not an image: {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn text_reads_to_its_own_limit_and_images_to_the_policys() {
        let cases = [
            (text("UTF8_STRING"), ImageCap(0), TEXT_LIMIT),
            (
                MimePick::Image("image/png".to_owned()),
                ImageCap(9),
                ReadLimit(9),
            ),
            (
                MimePick::Image("image/jpeg".to_owned()),
                ImageCap(0),
                ReadLimit(0),
            ),
        ];
        for (pick, images, want) in cases {
            assert_eq!(limit_for(&pick, images), want, "{pick:?}");
        }
    }
}
