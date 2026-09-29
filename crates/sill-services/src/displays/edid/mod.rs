//! A small EDID base-block parser: who made the display, what it calls itself, how big it is,
//! its native mode and its primaries. Pure: bytes in, [`Edid`] out. Extension blocks (CTA-861,
//! DisplayID) are not read; everything the shell needs is in the first 128 bytes.
//!
//! Hand-written rather than a crate: the base block is a fixed layout of a few dozen bytes, and
//! the maintained parsers either pull in a parser-combinator stack for it or bind libdisplay-info
//! through C.

mod chroma;
mod descriptors;
#[cfg(test)]
pub(crate) mod tests;

pub use chroma::{Chromaticity, Primaries};
pub use descriptors::{Millimetres, Mode, Pixels};

use serde::{Deserialize, Serialize};

use descriptors::Descriptor;

/// The three-letter PNP manufacturer id (`BNQ`, `DEL`, `SAM`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Manufacturer(pub String);

/// The manufacturer's product code (bytes 10-11, little-endian).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProductCode(pub u16);

/// The serial a display reports: the 0xFF descriptor's text when present, else the base block's
/// 32-bit number (often a placeholder such as `0x01010101`, which still tells two panels of one
/// model apart no worse than nothing).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Serial {
    Text(String),
    Number(u32),
}

/// What an EDID base block says about the display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edid {
    pub manufacturer: Manufacturer,
    pub product: ProductCode,
    pub serial: Serial,
    /// The 0xFC descriptor (`BenQ MA270U`); empty when there is none.
    pub name: String,
    /// The image size: the preferred timing's millimetres when given, else the base block's
    /// centimetres. `None` for a projector or a TV that reports 0 (or only an aspect ratio).
    pub size: Option<Millimetres>,
    /// The preferred (first) detailed timing: the panel's native mode.
    pub native: Option<Mode>,
    pub primaries: Primaries,
}

/// Why bytes are not an EDID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EdidError {
    #[error("shorter than one 128-byte block")]
    Short,
    #[error("no 00 FF FF FF FF FF FF 00 header")]
    Header,
    #[error("the base block's checksum does not add up")]
    Checksum,
}

const HEADER: [u8; 8] = [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];

/// Parse the base block at the start of `bytes` (a sysfs `edid` file, 128 bytes per block).
pub fn parse(bytes: &[u8]) -> Result<Edid, EdidError> {
    let block: &[u8; 128] = bytes
        .get(..128)
        .and_then(|b| b.try_into().ok())
        .ok_or(EdidError::Short)?;
    if block[..8] != HEADER {
        return Err(EdidError::Header);
    }
    if block.iter().fold(0u8, |sum, b| sum.wrapping_add(*b)) != 0 {
        return Err(EdidError::Checksum);
    }
    let found: Vec<Descriptor> = (0..4)
        .filter_map(|i| {
            let at = 54 + i * 18;
            block[at..at + 18].try_into().ok()
        })
        .map(|d: &[u8; 18]| descriptors::descriptor(d))
        .collect();
    let timing = found.iter().find_map(|d| match d {
        Descriptor::Timing(t) => Some(*t),
        _ => None,
    });
    let text = |want: fn(&Descriptor) -> Option<&String>| found.iter().find_map(want).cloned();
    let number = u32::from_le_bytes([block[12], block[13], block[14], block[15]]);
    Ok(Edid {
        manufacturer: manufacturer(block[8], block[9]),
        product: ProductCode(u16::from_le_bytes([block[10], block[11]])),
        serial: text(|d| match d {
            Descriptor::SerialText(s) if !s.is_empty() => Some(s),
            _ => None,
        })
        .map_or(Serial::Number(number), Serial::Text),
        name: text(|d| match d {
            Descriptor::ProductName(s) => Some(s),
            _ => None,
        })
        .unwrap_or_default(),
        size: timing
            .and_then(|t| t.image)
            .or_else(|| centimetres(block[21], block[22])),
        native: timing.map(|t| t.mode),
        primaries: chroma::primaries(block),
    })
}

/// Bytes 8-9, big-endian: three five-bit letters, 1 = `A`.
fn manufacturer(high: u8, low: u8) -> Manufacturer {
    let word = u16::from_be_bytes([high, low]);
    let letter = |shift: u16| {
        let code = ((word >> shift) & 0x1F) as u8;
        match code {
            1..=26 => char::from(b'A' + code - 1),
            _ => '?',
        }
    };
    Manufacturer([letter(10), letter(5), letter(0)].iter().collect())
}

/// Bytes 21-22: the maximum image size in centimetres. Either being zero means "unknown" (or,
/// in EDID 1.4, an aspect ratio rather than a size), which is no size at all here.
fn centimetres(width: u8, height: u8) -> Option<Millimetres> {
    (width > 0 && height > 0).then_some(Millimetres {
        width: u32::from(width) * 10,
        height: u32::from(height) * 10,
    })
}
