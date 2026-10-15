//! A whole output as it was shown at one moment, taken so the eyedropper can read a colour from
//! it and draw it still. Pure data.

use std::fmt;
use std::sync::Arc;

/// One output's picture: its pixels, for reading a colour, and the same picture as a PNG, for
/// drawing it.
#[derive(Clone, PartialEq, Eq)]
pub struct OutputShot {
    /// Pixels across and down, in device pixels (not the output's logical size).
    pub size: (u32, u32),
    /// The pixels as tightly packed RGBA8, rows top to bottom: `4 * width * height` bytes.
    pub rgba: Arc<[u8]>,
    /// The same picture, PNG-encoded.
    pub png: Arc<[u8]>,
}

// By hand: a derived `Debug` would print every byte of the picture into a log.
impl fmt::Debug for OutputShot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputShot")
            .field("size", &self.size)
            .field("rgba_bytes", &self.rgba.len())
            .field("png_bytes", &self.png.len())
            .finish()
    }
}

/// What a request for a picture of a whole output came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputPicture {
    /// The picture.
    Shot(OutputShot),
    /// The compositor could not make one, or not one upright.
    Failed,
}
