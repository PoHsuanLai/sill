//! `image-data`: the spec's `(iiibiiay)` read into straight RGBA, within the limits a card has
//! use for ([`IMAGE_MAX_SIDE`], [`IMAGE_MAX_BYTES`]). The limits are checked on the structure's
//! numbers and on the array's length before a byte is copied, so a huge array costs nothing more
//! than its arrival.

use sill_model::notifications::{IMAGE_MAX_BYTES, IMAGE_MAX_SIDE, Pixels};
use zbus::zvariant::Value;

/// `(iiibiiay)`: width, height, rowstride, has alpha, bits per sample, channels, data; 8-bit
/// RGB or RGBA only. `None` for anything malformed or past the limits.
pub(super) fn pixels(data: &Value<'_>) -> Option<Pixels> {
    let Value::Structure(fields) = data else {
        return None;
    };
    let [
        Value::I32(width),
        Value::I32(height),
        Value::I32(rowstride),
        Value::Bool(alpha),
        Value::I32(8),
        Value::I32(channels),
        Value::Array(bytes),
    ] = fields.fields()
    else {
        return None;
    };
    let shape = Shape {
        width: usize::try_from(*width).ok()?,
        height: usize::try_from(*height).ok()?,
        rowstride: usize::try_from(*rowstride).ok()?,
        channels: usize::try_from(*channels).ok()?,
        alpha: if *alpha { Alpha::Has } else { Alpha::None },
    };
    if !shape.within_limits() || bytes.inner().len() > IMAGE_MAX_BYTES {
        return None;
    }
    let bytes: Vec<u8> = bytes
        .inner()
        .iter()
        .map(|b| match b {
            Value::U8(b) => Some(*b),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let rgba = shape.rgba(&bytes)?;
    Some(Pixels {
        width: u32::try_from(shape.width).ok()?,
        height: u32::try_from(shape.height).ok()?,
        rgba: rgba.into(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Alpha {
    Has,
    None,
}

/// How `image-data`'s bytes are laid out.
#[derive(Debug, Clone, Copy)]
struct Shape {
    width: usize,
    height: usize,
    rowstride: usize,
    channels: usize,
    alpha: Alpha,
}

impl Shape {
    /// Whether the picture is no larger than [`IMAGE_MAX_SIDE`] either way.
    fn within_limits(self) -> bool {
        let side = usize::try_from(IMAGE_MAX_SIDE).unwrap_or(usize::MAX);
        self.width <= side && self.height <= side
    }

    /// The bytes as straight RGBA, rows packed; `None` when they are too few or the channels
    /// do not match.
    fn rgba(self, bytes: &[u8]) -> Option<Vec<u8>> {
        let wanted = match self.alpha {
            Alpha::Has => 4,
            Alpha::None => 3,
        };
        if self.channels != wanted || self.width == 0 || self.height == 0 {
            return None;
        }
        let row = self.width.checked_mul(self.channels)?;
        let last = self
            .rowstride
            .checked_mul(self.height - 1)?
            .checked_add(row)?;
        if self.rowstride < row || bytes.len() < last {
            return None;
        }
        let mut out = Vec::with_capacity(self.width * self.height * 4);
        for y in 0..self.height {
            let start = y * self.rowstride;
            for pixel in bytes[start..start + row].chunks_exact(self.channels) {
                out.extend_from_slice(&pixel[..3]);
                out.push(pixel.get(3).copied().unwrap_or(u8::MAX));
            }
        }
        Some(out)
    }
}
