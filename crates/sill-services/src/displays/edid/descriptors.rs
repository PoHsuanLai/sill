//! The four 18-byte descriptors of an EDID base block (bytes 54-125): detailed timings (the
//! first is the preferred, native mode, and carries the image size in millimetres) and display
//! descriptors (the product name, the serial string).

use serde::{Deserialize, Serialize};

/// A mode's pixel size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
}

/// A physical size in whole millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Millimetres {
    pub width: u32,
    pub height: u32,
}

/// One mode: pixels and refresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Mode {
    pub pixels: Pixels,
    /// Refresh rate in millihertz (`59997` for 59.997 Hz); 0 when unknown.
    pub refresh_mhz: u32,
}

/// One detailed timing descriptor, reduced to what the shell uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timing {
    pub mode: Mode,
    /// The addressable image size, when the descriptor gives one (both sides non-zero).
    pub image: Option<Millimetres>,
}

/// One descriptor, classified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Descriptor {
    Timing(Timing),
    /// Tag 0xFC.
    ProductName(String),
    /// Tag 0xFF.
    SerialText(String),
    /// Any other display descriptor, or an empty slot.
    Other,
}

/// The descriptor at `bytes` (18 of them).
pub(crate) fn descriptor(bytes: &[u8; 18]) -> Descriptor {
    let pixel_clock = u16::from_le_bytes([bytes[0], bytes[1]]);
    if pixel_clock != 0 {
        return Descriptor::Timing(timing(bytes, pixel_clock));
    }
    match bytes[3] {
        0xFC => Descriptor::ProductName(text(&bytes[5..])),
        0xFF => Descriptor::SerialText(text(&bytes[5..])),
        _ => Descriptor::Other,
    }
}

/// Twelve bits: the low eight in `low`, the high four in `nibbles`' upper (`Upper`) or lower
/// half.
fn twelve(low: u8, nibbles: u8, half: Half) -> u32 {
    let high = match half {
        Half::Upper => nibbles >> 4,
        Half::Lower => nibbles & 0x0F,
    };
    (u32::from(high) << 8) | u32::from(low)
}

/// Which nibble of a shared byte holds a field's high bits.
#[derive(Debug, Clone, Copy)]
enum Half {
    Upper,
    Lower,
}

fn timing(b: &[u8; 18], pixel_clock: u16) -> Timing {
    let width = twelve(b[2], b[4], Half::Upper);
    let h_blank = twelve(b[3], b[4], Half::Lower);
    let height = twelve(b[5], b[7], Half::Upper);
    let v_blank = twelve(b[6], b[7], Half::Lower);
    let image_w = twelve(b[12], b[14], Half::Upper);
    let image_h = twelve(b[13], b[14], Half::Lower);
    let total = u64::from(width + h_blank) * u64::from(height + v_blank);
    // The clock is in units of 10 kHz; millihertz is clock x 10^4 x 10^3 / pixels-per-frame.
    let refresh_mhz = (u64::from(pixel_clock) * 10_000_000)
        .checked_div(total)
        .map_or(0, |mhz| u32::try_from(mhz).unwrap_or(0));
    Timing {
        mode: Mode {
            pixels: Pixels { width, height },
            refresh_mhz,
        },
        image: (image_w > 0 && image_h > 0).then_some(Millimetres {
            width: image_w,
            height: image_h,
        }),
    }
}

/// A descriptor's text: up to 13 bytes of code page 437, ended by a line feed and padded with
/// spaces. Anything outside printable ASCII is dropped rather than guessed at.
fn text(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take_while(|&&b| b != 0x0A)
        .filter(|b| b.is_ascii_graphic() || **b == b' ')
        .map(|&b| char::from(b))
        .collect::<String>()
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{Descriptor, Millimetres, Mode, Pixels, Timing, descriptor};

    #[test]
    fn descriptors_classify_by_clock_and_tag() {
        let ma270u_timing: [u8; 18] = [
            0x4d, 0xd0, 0x00, 0xa0, 0xf0, 0x70, 0x3e, 0x80, 0x30, 0x20, 0x35, 0x00, 0x55, 0x50,
            0x21, 0x00, 0x00, 0x1a,
        ];
        let mut name = [0u8; 18];
        name[3] = 0xFC;
        name[5..18].copy_from_slice(b"BenQ MA270U\n ");
        let mut serial = [0u8; 18];
        serial[3] = 0xFF;
        serial[5..18].copy_from_slice(b"ABC123\n      ");
        let mut range = [0u8; 18];
        range[3] = 0xFD;
        let cases: Vec<(&str, [u8; 18], Descriptor)> = vec![
            (
                "the MA270U's preferred timing",
                ma270u_timing,
                Descriptor::Timing(Timing {
                    mode: Mode {
                        pixels: Pixels {
                            width: 3840,
                            height: 2160,
                        },
                        refresh_mhz: 59996,
                    },
                    image: Some(Millimetres {
                        width: 597,
                        height: 336,
                    }),
                }),
            ),
            (
                "a product name",
                name,
                Descriptor::ProductName("BenQ MA270U".to_owned()),
            ),
            (
                "a serial string",
                serial,
                Descriptor::SerialText("ABC123".to_owned()),
            ),
            ("a range-limits descriptor", range, Descriptor::Other),
        ];
        for (label, bytes, want) in cases {
            assert_eq!(descriptor(&bytes), want, "{label}");
        }
    }
}
